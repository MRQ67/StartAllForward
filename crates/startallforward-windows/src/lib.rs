pub mod api;
pub mod app_discovery;
pub mod appbar;
pub mod desktop;
pub mod enumerator;
pub mod filter;
pub mod hook;
pub mod icon;
pub mod launcher;
pub mod service;
pub mod settings_manager;
pub mod start_menu_window;
pub mod taskbar_window;

pub use api::{
    capture_window_state, get_foreground_hwnd, get_process_name, get_window_class_name,
    get_window_pid_tid, get_window_text, is_window_cloaked, is_window_foreground,
    is_window_minimized, is_window_taskbar_eligible, is_window_visible,
};
pub use app_discovery::{discover_installed_apps, get_known_folder_path, parse_shortcut};
pub use appbar::{AppBarEdge, AppBarManager, WM_APPBAR_CALLBACK};
pub use desktop::try_attach_default_desktop;
pub use enumerator::enumerate_desktop_windows;
pub use filter::is_taskbar_eligible_flags;
pub use hook::WinEventHookHandle;
pub use icon::{extract_window_icon, IconCache};
pub use launcher::{launch_app, launch_desktop_app, launch_packaged_app};
pub use service::WindowTrackingService;
pub use settings_manager::SettingsManager;
pub use start_menu_window::StartMenuWindow;
pub use taskbar_window::CustomTaskbar;
