//! Forcing the overlay window into the foreground.
//!
//! `ViewportCommand::Focus` boils down to `SetForegroundWindow`, which Windows
//! denies to background processes: if another process holds the foreground
//! lock when the hotkey fires (e.g. the Start menu is open), the overlay
//! appears but keystrokes keep going to the old window.
//!
//! Countermeasures, layered because no single one is reliable on Windows 11:
//! 1. If the foreground window belongs to a shell flyout (Start menu, Search,
//!    Action Center), tap ESC to dismiss it — these windows swallow keystrokes
//!    and hold the foreground lock aggressively.
//! 2. Tap ALT (press+release *before* `SetForegroundWindow`) to release the
//!    foreground lock, attach our input queue to the foreground thread, then
//!    call `SetForegroundWindow`/`BringWindowToTop`/`SetFocus` directly.
//! 3. Verify with `GetForegroundWindow` and retry a few times.

#[cfg(target_os = "windows")]
mod win {
    pub const VK_ESCAPE: u8 = 0x1B;
    pub const VK_MENU: u8 = 0x12;
    pub const KEYEVENTF_KEYUP: u32 = 0x0002;
    pub const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;

    #[link(name = "user32")]
    extern "system" {
        pub fn GetForegroundWindow() -> isize;
        pub fn SetForegroundWindow(hwnd: isize) -> i32;
        pub fn BringWindowToTop(hwnd: isize) -> i32;
        pub fn SetFocus(hwnd: isize) -> isize;
        pub fn AttachThreadInput(id_attach: u32, id_attach_to: u32, attach: i32) -> i32;
        pub fn GetWindowThreadProcessId(hwnd: isize, pid: *mut u32) -> u32;
        pub fn keybd_event(vk: u8, scan: u8, flags: u32, extra_info: usize);
    }
    #[link(name = "kernel32")]
    extern "system" {
        pub fn GetCurrentThreadId() -> u32;
        pub fn OpenProcess(access: u32, inherit: i32, pid: u32) -> isize;
        pub fn QueryFullProcessImageNameW(
            process: isize,
            flags: u32,
            name: *mut u16,
            size: *mut u32,
        ) -> i32;
        pub fn CloseHandle(handle: isize) -> i32;
    }

    pub unsafe fn tap_key(vk: u8) {
        keybd_event(vk, 0, 0, 0);
        keybd_event(vk, 0, KEYEVENTF_KEYUP, 0);
    }

    /// True if `hwnd` belongs to one of the shell flyout hosts (Start menu,
    /// Search, Action Center) that capture keyboard input while open.
    pub unsafe fn is_shell_flyout(hwnd: isize) -> bool {
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid == 0 {
            return false;
        }
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if process == 0 {
            return false;
        }
        let mut buf = [0u16; 512];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(process, 0, buf.as_mut_ptr(), &mut len) != 0;
        CloseHandle(process);
        if !ok {
            return false;
        }
        let path = String::from_utf16_lossy(&buf[..len as usize]).to_ascii_lowercase();
        [
            "startmenuexperiencehost.exe",
            "searchhost.exe",
            "searchapp.exe",
            "shellexperiencehost.exe",
        ]
        .iter()
        .any(|name| path.ends_with(name))
    }
}

#[cfg(target_os = "windows")]
pub fn force_foreground(frame: &eframe::Frame) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use win::*;

    let Ok(handle) = frame.window_handle() else {
        return;
    };
    let RawWindowHandle::Win32(win32) = handle.as_raw() else {
        return;
    };
    let hwnd = win32.hwnd.get();

    unsafe {
        let foreground = GetForegroundWindow();
        if foreground == hwnd {
            return;
        }

        // Dismiss an open shell flyout; it swallows keystrokes and refuses to
        // yield the foreground lock even to the tricks below.
        if is_shell_flyout(foreground) {
            tap_key(VK_ESCAPE);
            std::thread::sleep(std::time::Duration::from_millis(60));
        }

        for attempt in 0..3 {
            if attempt > 0 {
                std::thread::sleep(std::time::Duration::from_millis(30));
            }

            // An ALT tap makes the system release the foreground lock for the
            // next SetForegroundWindow call.
            tap_key(VK_MENU);

            let our_thread = GetCurrentThreadId();
            let fg_thread =
                GetWindowThreadProcessId(GetForegroundWindow(), std::ptr::null_mut());
            let attached = fg_thread != 0
                && fg_thread != our_thread
                && AttachThreadInput(our_thread, fg_thread, 1) != 0;

            SetForegroundWindow(hwnd);
            BringWindowToTop(hwnd);
            SetFocus(hwnd);

            if attached {
                AttachThreadInput(our_thread, fg_thread, 0);
            }

            if GetForegroundWindow() == hwnd {
                return;
            }
        }
        eprintln!("keykoff: failed to take foreground from window {:#x}", foreground);
    }
}

#[cfg(not(target_os = "windows"))]
pub fn force_foreground(_frame: &eframe::Frame) {}
