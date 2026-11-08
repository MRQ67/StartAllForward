use tracing::debug;
use windows::core::w;
use windows::Win32::System::StationsAndDesktops::{
    CloseDesktop, OpenDesktopW, SetThreadDesktop, DESKTOP_CONTROL_FLAGS,
};

const DESKTOP_ALL_ACCESS: u32 = 0x01FF;

/// Attempt to attach the calling thread to the interactive "Default" desktop.
/// This ensures window enumeration and WinEvent hooks can discover top-level
/// windows even when launched from secondary/subshell desktop sessions.
///
/// Must be called on a newly spawned thread before any hooks or windows are created.
pub fn try_attach_default_desktop() {
    unsafe {
        if let Ok(desk) = OpenDesktopW(
            w!("Default"),
            DESKTOP_CONTROL_FLAGS(0),
            false,
            DESKTOP_ALL_ACCESS,
        ) {
            if !desk.is_invalid() {
                if SetThreadDesktop(desk).is_ok() {
                    debug!("Thread successfully attached to Default desktop");
                }
                let _ = CloseDesktop(desk);
            }
        }
    }
}
