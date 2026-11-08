pub mod api;
pub mod desktop;
pub mod enumerator;
pub mod filter;
pub mod hook;
pub mod service;

pub use api::{
    capture_window_state, get_foreground_hwnd, get_process_name, get_window_class_name,
    get_window_pid_tid, get_window_text, is_window_cloaked, is_window_foreground,
    is_window_minimized, is_window_taskbar_eligible, is_window_visible,
};
pub use desktop::try_attach_default_desktop;
pub use enumerator::enumerate_desktop_windows;
pub use filter::is_taskbar_eligible_flags;
pub use hook::WinEventHookHandle;
pub use service::WindowTrackingService;
