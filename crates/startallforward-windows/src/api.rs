use std::path::Path;
use windows::core::PWSTR;
use windows::Win32::Foundation::{CloseHandle, HWND};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetClassNameW, GetForegroundWindow, GetWindow, GetWindowLongW, GetWindowTextLengthW,
    GetWindowTextW, GetWindowThreadProcessId, IsIconic, IsWindow, IsWindowVisible, GWL_EXSTYLE,
    GWL_STYLE, GW_OWNER,
};

use crate::filter::is_taskbar_eligible_flags;
use startallforward_core::{WindowId, WindowState};

/// Safe wrapper to retrieve window text / title.
pub fn get_window_text(hwnd: HWND) -> String {
    unsafe {
        if !IsWindow(Some(hwnd)).as_bool() {
            return String::new();
        }

        let len = GetWindowTextLengthW(hwnd);
        if len <= 0 {
            return String::new();
        }

        let mut buf = vec![0u16; (len + 1) as usize];
        let copied = GetWindowTextW(hwnd, &mut buf);
        if copied <= 0 {
            return String::new();
        }

        String::from_utf16_lossy(&buf[..copied as usize])
    }
}

/// Safe wrapper to retrieve the Win32 window class name.
pub fn get_window_class_name(hwnd: HWND) -> String {
    unsafe {
        if !IsWindow(Some(hwnd)).as_bool() {
            return String::new();
        }

        let mut buf = [0u16; 256];
        let copied = GetClassNameW(hwnd, &mut buf);
        if copied <= 0 {
            return String::new();
        }

        String::from_utf16_lossy(&buf[..copied as usize])
    }
}

/// Safe wrapper to retrieve the PID and Thread ID owning the window.
pub fn get_window_pid_tid(hwnd: HWND) -> (u32, u32) {
    unsafe {
        let mut pid: u32 = 0;
        let tid = GetWindowThreadProcessId(hwnd, Some(&mut pid));
        (pid, tid)
    }
}

/// Safe wrapper to query the executable process name from a PID.
pub fn get_process_name(pid: u32) -> String {
    if pid == 0 {
        return "System Idle Process".to_string();
    }
    if pid == 4 {
        return "System".to_string();
    }

    unsafe {
        let handle = match OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
            Ok(h) if !h.is_invalid() => h,
            _ => return "(inaccessible)".to_string(),
        };

        let mut buf = [0u16; 1024];
        let mut size = buf.len() as u32;

        let result = QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            PWSTR(buf.as_mut_ptr()),
            &mut size,
        );

        let _ = CloseHandle(handle);

        if result.is_ok() && size > 0 {
            let full_path = String::from_utf16_lossy(&buf[..size as usize]);
            Path::new(&full_path)
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or(full_path)
        } else {
            "(inaccessible)".to_string()
        }
    }
}

/// Safe wrapper to check if a window is visible.
pub fn is_window_visible(hwnd: HWND) -> bool {
    unsafe { IsWindowVisible(hwnd).as_bool() }
}

/// Safe wrapper to check if a window is minimized (iconic).
pub fn is_window_minimized(hwnd: HWND) -> bool {
    unsafe { IsIconic(hwnd).as_bool() }
}

/// Safe wrapper to check if a window is cloaked (e.g. suspended UWP, on another virtual desktop).
pub fn is_window_cloaked(hwnd: HWND) -> bool {
    unsafe {
        let mut cloaked: u32 = 0;
        let hr = DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED,
            &mut cloaked as *mut _ as *mut _,
            std::mem::size_of::<u32>() as u32,
        );
        hr.is_ok() && cloaked != 0
    }
}

/// Query whether the window is currently the foreground active window.
pub fn is_window_foreground(hwnd: HWND) -> bool {
    unsafe {
        let fg = GetForegroundWindow();
        fg == hwnd
    }
}

/// Query the current foreground HWND.
pub fn get_foreground_hwnd() -> Option<HWND> {
    unsafe {
        let fg = GetForegroundWindow();
        if fg.is_invalid() || !IsWindow(Some(fg)).as_bool() {
            None
        } else {
            Some(fg)
        }
    }
}

/// Determine whether the window qualifies as a taskbar-eligible application window.
pub fn is_window_taskbar_eligible(hwnd: HWND) -> bool {
    unsafe {
        if !IsWindow(Some(hwnd)).as_bool() {
            return false;
        }

        let style = GetWindowLongW(hwnd, GWL_STYLE) as u32;
        let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
        let owner = GetWindow(hwnd, GW_OWNER).unwrap_or_default();
        let has_owner = !owner.is_invalid();
        let visible = IsWindowVisible(hwnd).as_bool();

        is_taskbar_eligible_flags(style, ex_style, has_owner, visible)
    }
}

/// Extract a complete `WindowState` snapshot from a live HWND.
pub fn capture_window_state(hwnd: HWND, now_ms: u64) -> Option<WindowState> {
    unsafe {
        if !IsWindow(Some(hwnd)).as_bool() {
            return None;
        }

        let (pid, _tid) = get_window_pid_tid(hwnd);
        let process_name = get_process_name(pid);
        let title = get_window_text(hwnd);
        let class_name = get_window_class_name(hwnd);
        let is_visible = is_window_visible(hwnd);
        let is_minimized = is_window_minimized(hwnd);
        let is_foreground = is_window_foreground(hwnd);
        let is_cloaked = is_window_cloaked(hwnd);
        let is_taskbar_eligible = is_window_taskbar_eligible(hwnd);

        Some(WindowState {
            id: WindowId::from(hwnd.0 as isize),
            title,
            process_id: pid,
            process_name,
            class_name,
            is_visible,
            is_minimized,
            is_foreground,
            is_cloaked,
            is_taskbar_eligible,
            first_seen_epoch_ms: now_ms,
            last_seen_epoch_ms: now_ms,
        })
    }
}
