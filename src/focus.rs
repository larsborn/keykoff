//! Forcing the overlay window into the foreground.
//!
//! `ViewportCommand::Focus` boils down to `SetForegroundWindow`, which Windows
//! denies to background processes: if another process holds the foreground
//! lock when the hotkey fires (e.g. the Start menu is open), the overlay
//! appears but keystrokes keep going to the old window.
//!
//! Usually no force is needed: the hotkey that summoned us credits our process
//! with the last input event, which is one of the documented conditions under
//! which Windows grants `SetForegroundWindow`. So we try the plain call first
//! and only escalate when it is actually denied — synthesizing input on every
//! summon is both unnecessary and a behaviour antivirus heuristics dislike.
//!
//! When the plain call *is* denied, the countermeasures are layered, because no
//! single one is reliable on Windows 11:
//! 1. If the foreground window belongs to a shell flyout (Start menu, Search,
//!    Action Center), tap ESC to dismiss it — these windows swallow keystrokes
//!    and hold the foreground lock aggressively.
//! 2. Tap ALT (press+release *before* `SetForegroundWindow`) to release the
//!    foreground lock, attach our input queue to the foreground thread, then
//!    call `SetForegroundWindow`/`BringWindowToTop`/`SetFocus` directly.
//! 3. Verify with `GetForegroundWindow` and retry a few times.

#[cfg(target_os = "windows")]
mod win {
    pub const VK_ESCAPE: u16 = 0x1B;
    pub const VK_SHIFT: u16 = 0x10;
    pub const VK_CONTROL: u16 = 0x11;
    pub const VK_MENU: u16 = 0x12;
    pub const VK_LWIN: u16 = 0x5B;
    pub const VK_RWIN: u16 = 0x5C;
    pub const KEYEVENTF_KEYUP: u32 = 0x0002;
    pub const INPUT_KEYBOARD: u32 = 1;
    pub const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;

    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct KeyboardInput {
        pub vk: u16,
        pub scan: u16,
        pub flags: u32,
        pub time: u32,
        pub extra_info: usize,
    }

    /// Present only for its size. `INPUT`'s union is as large as its largest
    /// member, and `MOUSEINPUT` is larger than `KEYBDINPUT`; `SendInput`
    /// rejects a `cb_size` that isn't exactly `sizeof(INPUT)`.
    #[repr(C)]
    #[derive(Clone, Copy)]
    #[allow(dead_code)]
    pub struct MouseInput {
        pub dx: i32,
        pub dy: i32,
        pub mouse_data: u32,
        pub flags: u32,
        pub time: u32,
        pub extra_info: usize,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    pub union InputPayload {
        pub keyboard: KeyboardInput,
        pub _mouse: MouseInput,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct Input {
        pub kind: u32,
        pub payload: InputPayload,
    }

    #[link(name = "user32")]
    extern "system" {
        pub fn GetForegroundWindow() -> isize;
        pub fn SetForegroundWindow(hwnd: isize) -> i32;
        pub fn BringWindowToTop(hwnd: isize) -> i32;
        pub fn SetFocus(hwnd: isize) -> isize;
        pub fn AttachThreadInput(id_attach: u32, id_attach_to: u32, attach: i32) -> i32;
        pub fn GetWindowThreadProcessId(hwnd: isize, pid: *mut u32) -> u32;
        pub fn SendInput(count: u32, inputs: *const Input, cb_size: i32) -> u32;
        pub fn GetAsyncKeyState(vk: i32) -> i16;
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

    /// Send a batch of `(virtual key, flags)` events in one `SendInput` call so
    /// nothing can interleave between them.
    unsafe fn send_keys(events: &[(u16, u32)]) {
        let inputs: Vec<Input> = events
            .iter()
            .map(|&(vk, flags)| Input {
                kind: INPUT_KEYBOARD,
                payload: InputPayload {
                    keyboard: KeyboardInput {
                        vk,
                        scan: 0,
                        flags,
                        time: 0,
                        extra_info: 0,
                    },
                },
            })
            .collect();
        SendInput(
            inputs.len() as u32,
            inputs.as_ptr(),
            std::mem::size_of::<Input>() as i32,
        );
    }

    pub unsafe fn tap_key(vk: u16) {
        send_keys(&[(vk, 0), (vk, KEYEVENTF_KEYUP)]);
    }

    /// Inject key-ups for any physically held modifiers. The hotkey that
    /// summoned us is still being held (e.g. CTRL of CTRL+F10), so without
    /// this the taps below become chords: CTRL+ALT doesn't release the
    /// foreground lock, and CTRL+ESC *toggles the Start menu* instead of
    /// dismissing it. The user's later physical release just produces a
    /// redundant key-up, which is harmless.
    pub unsafe fn release_held_modifiers() {
        let held: Vec<(u16, u32)> = [VK_CONTROL, VK_SHIFT, VK_MENU, VK_LWIN, VK_RWIN]
            .into_iter()
            .filter(|&vk| GetAsyncKeyState(vk as i32) as u16 & 0x8000 != 0)
            .map(|vk| (vk, KEYEVENTF_KEYUP))
            .collect();
        if !held.is_empty() {
            send_keys(&held);
        }
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

/// Plain activation, no input synthesis. Returns whether we actually ended up
/// in the foreground — `SetForegroundWindow`'s own return value can't be
/// trusted here, so `GetForegroundWindow` is the verdict.
#[cfg(target_os = "windows")]
unsafe fn activate(hwnd: isize) -> bool {
    use win::*;
    SetForegroundWindow(hwnd);
    BringWindowToTop(hwnd);
    SetFocus(hwnd);
    GetForegroundWindow() == hwnd
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

        // Fast path: nothing is contesting the foreground lock, so the plain
        // call is granted and no input has to be synthesized at all. This is
        // the normal case for a hotkey-summoned overlay.
        if activate(hwnd) {
            return;
        }

        // Denied — escalate. Logged so a debug run shows which path was taken;
        // release builds have no console.
        eprintln!("keykoff: foreground denied, escalating");

        // Release the hotkey's own modifiers first, or the taps below turn into
        // chords (CTRL+ALT doesn't release the foreground lock, and CTRL+ESC
        // *opens* the Start menu).
        release_held_modifiers();

        // Dismiss an open shell flyout; it swallows keystrokes and refuses to
        // yield the foreground lock even to the tricks below.
        if is_shell_flyout(GetForegroundWindow()) {
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

            let ok = activate(hwnd);

            if attached {
                AttachThreadInput(our_thread, fg_thread, 0);
            }

            if ok {
                return;
            }
        }
        eprintln!("keykoff: failed to take foreground from window {:#x}", foreground);
    }
}

#[cfg(not(target_os = "windows"))]
pub fn force_foreground(_frame: &eframe::Frame) {}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    #[test]
    fn input_layout_matches_win32() {
        // SendInput silently does nothing if cb_size isn't exactly
        // sizeof(INPUT): 40 bytes on 64-bit, 28 on 32-bit.
        let expected = if cfg!(target_pointer_width = "64") { 40 } else { 28 };
        assert_eq!(std::mem::size_of::<super::win::Input>(), expected);
    }
}
