use std::collections::HashMap;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    GetClassLongPtrW, SendMessageTimeoutW, GCLP_HICON, GCLP_HICONSM, HICON, ICON_BIG, ICON_SMALL,
    ICON_SMALL2, SMTO_ABORTIFHUNG, WM_GETICON,
};

use startallforward_core::WindowId;

/// Cache for extracted application icons to avoid continuous IPC message spam.
pub struct IconCache {
    icons: HashMap<WindowId, HICON>,
}

impl Default for IconCache {
    fn default() -> Self {
        Self::new()
    }
}

impl IconCache {
    pub fn new() -> Self {
        Self {
            icons: HashMap::new(),
        }
    }

    /// Retrieve an HICON for the target window, querying Win32 APIs if not cached.
    pub fn get_or_extract(&mut self, hwnd: HWND) -> Option<HICON> {
        let id = WindowId::from(hwnd.0 as isize);
        if let Some(&icon) = self.icons.get(&id) {
            if !icon.is_invalid() {
                return Some(icon);
            }
        }

        let icon = extract_window_icon(hwnd)?;
        self.icons.insert(id, icon);
        Some(icon)
    }

    /// Invalidate/remove an icon when a window is closed.
    pub fn remove(&mut self, id: WindowId) {
        self.icons.remove(&id);
    }
}

/// Safely extract an HICON from a live top-level HWND using standard Win32 queries.
pub fn extract_window_icon(hwnd: HWND) -> Option<HICON> {
    if hwnd.is_invalid() {
        return None;
    }

    unsafe {
        let mut result: usize = 0;

        // 1. Try WM_GETICON (ICON_SMALL2)
        let sent = SendMessageTimeoutW(
            hwnd,
            WM_GETICON,
            WPARAM(ICON_SMALL2 as usize),
            LPARAM(0),
            SMTO_ABORTIFHUNG,
            50,
            Some(&mut result),
        );
        if sent.0 != 0 && result != 0 {
            let hicon = HICON(result as *mut _);
            if !hicon.is_invalid() {
                return Some(hicon);
            }
        }

        // 2. Try WM_GETICON (ICON_SMALL)
        let sent = SendMessageTimeoutW(
            hwnd,
            WM_GETICON,
            WPARAM(ICON_SMALL as usize),
            LPARAM(0),
            SMTO_ABORTIFHUNG,
            50,
            Some(&mut result),
        );
        if sent.0 != 0 && result != 0 {
            let hicon = HICON(result as *mut _);
            if !hicon.is_invalid() {
                return Some(hicon);
            }
        }

        // 3. Try WM_GETICON (ICON_BIG)
        let sent = SendMessageTimeoutW(
            hwnd,
            WM_GETICON,
            WPARAM(ICON_BIG as usize),
            LPARAM(0),
            SMTO_ABORTIFHUNG,
            50,
            Some(&mut result),
        );
        if sent.0 != 0 && result != 0 {
            let hicon = HICON(result as *mut _);
            if !hicon.is_invalid() {
                return Some(hicon);
            }
        }

        // 4. Try window class small icon (GCLP_HICONSM)
        let class_icon_sm = GetClassLongPtrW(hwnd, GCLP_HICONSM);
        if class_icon_sm != 0 {
            let hicon = HICON(class_icon_sm as *mut _);
            if !hicon.is_invalid() {
                return Some(hicon);
            }
        }

        // 5. Try window class default icon (GCLP_HICON)
        let class_icon = GetClassLongPtrW(hwnd, GCLP_HICON);
        if class_icon != 0 {
            let hicon = HICON(class_icon as *mut _);
            if !hicon.is_invalid() {
                return Some(hicon);
            }
        }

        None
    }
}
