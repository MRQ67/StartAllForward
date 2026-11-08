use crate::api::capture_window_state;
use startallforward_core::WindowState;
use windows::core::BOOL;
use windows::Win32::Foundation::{HWND, LPARAM};
use windows::Win32::UI::WindowsAndMessaging::EnumWindows;

struct EnumContext {
    windows: Vec<WindowState>,
    now_ms: u64,
}

/// Enumerate all top-level desktop windows and produce full snapshots.
pub fn enumerate_desktop_windows(now_ms: u64) -> Vec<WindowState> {
    let mut context = EnumContext {
        windows: Vec::new(),
        now_ms,
    };
    let lparam = LPARAM(&mut context as *mut EnumContext as isize);

    unsafe {
        let _ = EnumWindows(Some(enum_windows_callback), lparam);
    }

    context.windows
}

unsafe extern "system" fn enum_windows_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let ctx = &mut *(lparam.0 as *mut EnumContext);

    // Ignore null or invalid window handles
    if hwnd.is_invalid() {
        return BOOL(1);
    }

    if let Some(state) = capture_window_state(hwnd, ctx.now_ms) {
        ctx.windows.push(state);
    }

    BOOL(1)
}
