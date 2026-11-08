use tracing::{debug, error, info, warn};
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTOPRIMARY,
};
use windows::Win32::UI::Shell::{
    SHAppBarMessage, ABE_BOTTOM, ABE_TOP, ABM_ACTIVATE, ABM_NEW, ABM_QUERYPOS, ABM_REMOVE,
    ABM_SETPOS, ABM_WINDOWPOSCHANGED, ABN_POSCHANGED, APPBARDATA,
};
use windows::Win32::UI::WindowsAndMessaging::{
    SetWindowPos, HWND_TOPMOST, SWP_NOACTIVATE, SWP_SHOWWINDOW, WM_USER,
};

/// Custom window message sent by the shell to our AppBar window.
pub const WM_APPBAR_CALLBACK: u32 = WM_USER + 101;

/// Screen edge docking position for the custom taskbar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AppBarEdge {
    #[default]
    Top,
    Bottom,
}

/// Manager for Windows Application Desktop Toolbar (AppBar) registration,
/// workspace reservation, edge docking, and dynamic repositioning.
pub struct AppBarManager {
    hwnd: HWND,
    edge: AppBarEdge,
    thickness_px: i32,
    is_registered: bool,
    current_rect: RECT,
}

impl AppBarManager {
    /// Register a new AppBar with the Windows Shell.
    pub fn register(hwnd: HWND, edge: AppBarEdge, thickness_px: i32) -> Result<Self, String> {
        let mut abd = APPBARDATA {
            cbSize: std::mem::size_of::<APPBARDATA>() as u32,
            hWnd: hwnd,
            uCallbackMessage: WM_APPBAR_CALLBACK,
            ..Default::default()
        };

        let result = unsafe { SHAppBarMessage(ABM_NEW, &mut abd) };
        if result == 0 {
            error!("SHAppBarMessage(ABM_NEW) failed for HWND {:?}", hwnd);
            return Err("Failed to register AppBar with Windows Shell".to_string());
        }

        info!(
            "AppBar registered successfully for HWND {:?} on edge {:?}",
            hwnd, edge
        );

        let mut manager = Self {
            hwnd,
            edge,
            thickness_px,
            is_registered: true,
            current_rect: RECT::default(),
        };

        // Perform initial positioning and workspace reservation
        manager.reposition()?;
        Ok(manager)
    }

    /// Negotiate workspace coordinates with the Windows Shell and move the window.
    pub fn reposition(&mut self) -> Result<RECT, String> {
        if !self.is_registered {
            return Err("Cannot reposition an unregistered AppBar".to_string());
        }

        unsafe {
            // 1. Get primary monitor boundaries
            let hmonitor = MonitorFromWindow(self.hwnd, MONITOR_DEFAULTTOPRIMARY);
            let mut mi = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            if !GetMonitorInfoW(hmonitor, &mut mi).as_bool() {
                return Err("Failed to query monitor info".to_string());
            }

            let mon_rc = mi.rcMonitor;

            // 2. Formulate proposed bounding box based on edge
            let proposed = RECT {
                left: mon_rc.left,
                right: mon_rc.right,
                top: match self.edge {
                    AppBarEdge::Top => mon_rc.top,
                    AppBarEdge::Bottom => mon_rc.bottom - self.thickness_px,
                },
                bottom: match self.edge {
                    AppBarEdge::Top => mon_rc.top + self.thickness_px,
                    AppBarEdge::Bottom => mon_rc.bottom,
                },
            };

            let u_edge = match self.edge {
                AppBarEdge::Top => ABE_TOP,
                AppBarEdge::Bottom => ABE_BOTTOM,
            };

            let mut abd = APPBARDATA {
                cbSize: std::mem::size_of::<APPBARDATA>() as u32,
                hWnd: self.hwnd,
                uEdge: u_edge,
                rc: proposed,
                ..Default::default()
            };

            // 3. Query system if position is acceptable (shell may adjust for existing bars)
            let _ = SHAppBarMessage(ABM_QUERYPOS, &mut abd);

            // 4. Retain intended thickness after query adjustment
            match self.edge {
                AppBarEdge::Top => {
                    abd.rc.bottom = abd.rc.top + self.thickness_px;
                }
                AppBarEdge::Bottom => {
                    abd.rc.top = abd.rc.bottom - self.thickness_px;
                }
            }

            // 5. Commit and reserve desktop workspace
            let _ = SHAppBarMessage(ABM_SETPOS, &mut abd);

            self.current_rect = abd.rc;

            let width = abd.rc.right - abd.rc.left;
            let height = abd.rc.bottom - abd.rc.top;

            debug!(
                "AppBar committed rect: ({}, {}) - {}x{} (Edge: {:?})",
                abd.rc.left, abd.rc.top, width, height, self.edge
            );

            // 6. Move and resize the actual native window
            let moved = SetWindowPos(
                self.hwnd,
                Some(HWND_TOPMOST),
                abd.rc.left,
                abd.rc.top,
                width,
                height,
                SWP_NOACTIVATE | SWP_SHOWWINDOW,
            );

            if !moved.is_ok() {
                warn!("SetWindowPos failed during AppBar repositioning");
            }

            Ok(abd.rc)
        }
    }

    /// Notify the shell when window activation status changes.
    pub fn on_activate(&self) {
        if !self.is_registered {
            return;
        }
        let mut abd = APPBARDATA {
            cbSize: std::mem::size_of::<APPBARDATA>() as u32,
            hWnd: self.hwnd,
            ..Default::default()
        };
        unsafe {
            let _ = SHAppBarMessage(ABM_ACTIVATE, &mut abd);
        }
    }

    /// Notify the shell when window pos changed.
    pub fn on_window_pos_changed(&self) {
        if !self.is_registered {
            return;
        }
        let mut abd = APPBARDATA {
            cbSize: std::mem::size_of::<APPBARDATA>() as u32,
            hWnd: self.hwnd,
            ..Default::default()
        };
        unsafe {
            let _ = SHAppBarMessage(ABM_WINDOWPOSCHANGED, &mut abd);
        }
    }

    /// Handle system AppBar notifications (`ABN_POSCHANGED`, etc.).
    pub fn on_appbar_callback(&mut self, wparam: usize) {
        if wparam as u32 == ABN_POSCHANGED {
            debug!("Received ABN_POSCHANGED; recalculating AppBar position...");
            let _ = self.reposition();
        }
    }

    /// Update taskbar edge (Top/Bottom) and recalculate position.
    pub fn set_edge(&mut self, edge: AppBarEdge) -> Result<RECT, String> {
        self.edge = edge;
        self.reposition()
    }

    /// Update taskbar thickness (DPI scaling) and recalculate position.
    pub fn set_thickness(&mut self, thickness_px: i32) -> Result<RECT, String> {
        self.thickness_px = thickness_px;
        self.reposition()
    }

    /// Unregister from the shell and release the reserved workspace.
    pub fn unregister(&mut self) {
        if self.is_registered {
            info!("Unregistering AppBar for HWND {:?}", self.hwnd);
            let mut abd = APPBARDATA {
                cbSize: std::mem::size_of::<APPBARDATA>() as u32,
                hWnd: self.hwnd,
                ..Default::default()
            };
            unsafe {
                let _ = SHAppBarMessage(ABM_REMOVE, &mut abd);
            }
            self.is_registered = false;
        }
    }

    /// Return current reserved bounding rectangle.
    pub fn current_rect(&self) -> RECT {
        self.current_rect
    }

    /// Return active docking edge.
    pub fn edge(&self) -> AppBarEdge {
        self.edge
    }
}

impl Drop for AppBarManager {
    fn drop(&mut self) {
        self.unregister();
    }
}
