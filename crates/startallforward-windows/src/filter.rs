/// Logic for determining whether a top-level window qualifies for presentation
/// on the taskbar versus being an invisible tool/helper/overlay window.
///
/// Implemented according to classic Win32 Shell specifications:
/// 1. Window must be visible (`WS_VISIBLE`).
/// 2. Window must not be a child window (`WS_CHILD`).
/// 3. If `WS_EX_TOOLWINDOW` is set, it does NOT appear on the taskbar unless overridden by `WS_EX_APPWINDOW`.
/// 4. If the window has an owner window (`GW_OWNER`), it does NOT appear on the taskbar unless overridden by `WS_EX_APPWINDOW`.
pub fn is_taskbar_eligible_flags(
    style: u32,
    ex_style: u32,
    has_owner: bool,
    is_visible: bool,
) -> bool {
    const WS_CHILD: u32 = 0x40000000;
    const WS_EX_TOOLWINDOW: u32 = 0x00000080;
    const WS_EX_APPWINDOW: u32 = 0x00040000;

    if !is_visible {
        return false;
    }

    // Child windows belong to parent client areas, not the taskbar
    if (style & WS_CHILD) != 0 {
        return false;
    }

    let has_app_window = (ex_style & WS_EX_APPWINDOW) != 0;
    let has_tool_window = (ex_style & WS_EX_TOOLWINDOW) != 0;

    // Tool windows are excluded unless explicitly marked as app windows
    if has_tool_window && !has_app_window {
        return false;
    }

    // Owned windows (popups, dialogs) do not get their own taskbar button unless WS_EX_APPWINDOW is set
    if has_owner && !has_app_window {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normal_toplevel_window() {
        const WS_OVERLAPPEDWINDOW: u32 = 0x00CF0000;
        assert!(is_taskbar_eligible_flags(
            WS_OVERLAPPEDWINDOW,
            0,
            false,
            true
        ));
    }

    #[test]
    fn test_invisible_window() {
        const WS_OVERLAPPEDWINDOW: u32 = 0x00CF0000;
        assert!(!is_taskbar_eligible_flags(
            WS_OVERLAPPEDWINDOW,
            0,
            false,
            false
        ));
    }

    #[test]
    fn test_child_window() {
        const WS_CHILD: u32 = 0x40000000;
        assert!(!is_taskbar_eligible_flags(WS_CHILD, 0, false, true));
    }

    #[test]
    fn test_tool_window_without_appwindow() {
        const WS_EX_TOOLWINDOW: u32 = 0x00000080;
        assert!(!is_taskbar_eligible_flags(0, WS_EX_TOOLWINDOW, false, true));
    }

    #[test]
    fn test_tool_window_with_appwindow() {
        const WS_EX_TOOLWINDOW: u32 = 0x00000080;
        const WS_EX_APPWINDOW: u32 = 0x00040000;
        assert!(is_taskbar_eligible_flags(
            0,
            WS_EX_TOOLWINDOW | WS_EX_APPWINDOW,
            false,
            true
        ));
    }

    #[test]
    fn test_owned_window_without_appwindow() {
        assert!(!is_taskbar_eligible_flags(0, 0, true, true));
    }

    #[test]
    fn test_owned_window_with_appwindow() {
        const WS_EX_APPWINDOW: u32 = 0x00040000;
        assert!(is_taskbar_eligible_flags(0, WS_EX_APPWINDOW, true, true));
    }
}
