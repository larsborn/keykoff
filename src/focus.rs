//! Forcing the overlay window into the foreground.
//!
//! `ViewportCommand::Focus` boils down to `SetForegroundWindow`, which Windows
//! denies to background processes: if another process holds the foreground
//! lock when the hotkey fires (e.g. the Start menu is open), the overlay
//! appears but keystrokes keep going to the old window. The standard launcher
//! workaround is to simulate an ALT keypress (which releases the foreground
//! lock) and attach our input queue to the foreground thread, then call
//! `SetForegroundWindow` ourselves.

#[cfg(target_os = "windows")]
pub fn force_foreground(frame: &eframe::Frame) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    const VK_MENU: u8 = 0x12;
    const KEYEVENTF_KEYUP: u32 = 0x0002;

    #[link(name = "user32")]
    extern "system" {
        fn GetForegroundWindow() -> isize;
        fn SetForegroundWindow(hwnd: isize) -> i32;
        fn BringWindowToTop(hwnd: isize) -> i32;
        fn SetFocus(hwnd: isize) -> isize;
        fn AttachThreadInput(id_attach: u32, id_attach_to: u32, attach: i32) -> i32;
        fn GetWindowThreadProcessId(hwnd: isize, pid: *mut u32) -> u32;
        fn keybd_event(vk: u8, scan: u8, flags: u32, extra_info: usize);
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GetCurrentThreadId() -> u32;
    }

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

        // A (simulated) ALT press makes the system release the foreground
        // lock so SetForegroundWindow is honored; released again below.
        keybd_event(VK_MENU, 0, 0, 0);

        let our_thread = GetCurrentThreadId();
        let fg_thread = GetWindowThreadProcessId(foreground, std::ptr::null_mut());
        let attached = fg_thread != 0
            && fg_thread != our_thread
            && AttachThreadInput(our_thread, fg_thread, 1) != 0;

        SetForegroundWindow(hwnd);
        BringWindowToTop(hwnd);
        SetFocus(hwnd);

        if attached {
            AttachThreadInput(our_thread, fg_thread, 0);
        }
        keybd_event(VK_MENU, 0, KEYEVENTF_KEYUP, 0);
    }
}

#[cfg(not(target_os = "windows"))]
pub fn force_foreground(_frame: &eframe::Frame) {}
