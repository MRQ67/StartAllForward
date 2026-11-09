use crossbeam_channel::Receiver;
use std::time::SystemTime;
use tracing::{error, info, warn};
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, BitBlt, ClientToScreen, CreateCompatibleBitmap, CreateCompatibleDC, CreateFontW,
    CreateSolidBrush, DeleteDC, DeleteObject, DrawTextW, EndPaint, FillRect, InvalidateRect,
    SelectObject, SetBkMode, SetTextColor, DT_CENTER, DT_END_ELLIPSIS, DT_LEFT, DT_SINGLELINE,
    DT_VCENTER, FONT_CHARSET, FONT_CLIP_PRECISION, FONT_OUTPUT_PRECISION, FONT_QUALITY, FW_NORMAL,
    FW_SEMIBOLD, HBRUSH, HFONT, PAINTSTRUCT, SRCCOPY, TRANSPARENT,
};
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, DrawIconEx, GetClientRect,
    GetCursorPos, GetForegroundWindow, GetMessageW, GetWindowLongPtrW, GetWindowRect, IsIconic,
    IsWindow, KillTimer, LoadCursorW, PostMessageW, PostQuitMessage, RegisterClassExW,
    SetForegroundWindow, SetTimer, SetWindowLongPtrW, ShowWindowAsync, CREATESTRUCTW, CS_HREDRAW,
    CS_VREDRAW, DI_NORMAL, GWLP_USERDATA, HICON, HTCLIENT, IDC_ARROW, MA_NOACTIVATE, SW_MINIMIZE,
    SW_RESTORE, WM_ACTIVATE, WM_CLOSE, WM_CREATE, WM_DESTROY, WM_DISPLAYCHANGE, WM_DPICHANGED,
    WM_ERASEBKGND, WM_LBUTTONUP, WM_MOUSEACTIVATE, WM_MOUSEMOVE, WM_NCHITTEST, WM_PAINT, WM_TIMER,
    WM_WINDOWPOSCHANGED, WNDCLASSEXW, WS_CLIPCHILDREN, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
    WS_VISIBLE,
};

use crate::appbar::{AppBarEdge, AppBarManager, WM_APPBAR_CALLBACK};
use crate::icon::IconCache;
use crate::start_menu_window::StartMenuWindow;
use startallforward_core::{WindowEvent, WindowId, WindowTracker};

const TIMER_REFRESH_ID: usize = 1001;
const DEFAULT_TASKBAR_HEIGHT: i32 = 44;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ButtonTarget {
    Start,
    Window(WindowId),
}

struct ButtonRect {
    target: ButtonTarget,
    rect: RECT,
}

/// Internal state for the custom AppBar taskbar window.
pub struct TaskbarState {
    hwnd: HWND,
    appbar: Option<AppBarManager>,
    icon_cache: IconCache,
    tracker: WindowTracker,
    event_rx: Receiver<WindowEvent>,
    edge: AppBarEdge,
    thickness_px: i32,
    hovered_target: Option<ButtonTarget>,
    button_layout: Vec<ButtonRect>,
    font_main: HFONT,
    font_bold: HFONT,
    font_clock: HFONT,
    brush_bg: HBRUSH,
    brush_btn_normal: HBRUSH,
    brush_btn_hover: HBRUSH,
    brush_btn_active: HBRUSH,
    brush_start_normal: HBRUSH,
    brush_start_hover: HBRUSH,
    brush_accent: HBRUSH,
    start_menu: Option<StartMenuWindow>,
    start_btn_rect: RECT,
}

impl TaskbarState {
    pub fn new(
        hwnd: HWND,
        edge: AppBarEdge,
        thickness_px: i32,
        event_rx: Receiver<WindowEvent>,
    ) -> Self {
        unsafe {
            let dpi = GetDpiForWindow(hwnd);
            let font_height = -((12 * dpi as i32) / 96);
            let clock_font_height = -((11 * dpi as i32) / 96);

            let font_main = CreateFontW(
                font_height,
                0,
                0,
                0,
                FW_NORMAL.0 as i32,
                0,
                0,
                0,
                FONT_CHARSET(0),
                FONT_OUTPUT_PRECISION(0),
                FONT_CLIP_PRECISION(0),
                FONT_QUALITY(0),
                0,
                w!("Segoe UI"),
            );
            let font_bold = CreateFontW(
                font_height,
                0,
                0,
                0,
                FW_SEMIBOLD.0 as i32,
                0,
                0,
                0,
                FONT_CHARSET(0),
                FONT_OUTPUT_PRECISION(0),
                FONT_CLIP_PRECISION(0),
                FONT_QUALITY(0),
                0,
                w!("Segoe UI"),
            );
            let font_clock = CreateFontW(
                clock_font_height,
                0,
                0,
                0,
                FW_NORMAL.0 as i32,
                0,
                0,
                0,
                FONT_CHARSET(0),
                FONT_OUTPUT_PRECISION(0),
                FONT_CLIP_PRECISION(0),
                FONT_QUALITY(0),
                0,
                w!("Segoe UI"),
            );

            // Modern Dark Visual Palette (Windows 11 inspired)
            // ColorRef is 0x00BBGGRR
            let brush_bg = CreateSolidBrush(COLORREF(0x001B1818)); // RGB(24, 24, 27)
            let brush_btn_normal = CreateSolidBrush(COLORREF(0x00272322)); // RGB(34, 35, 39)
            let brush_btn_hover = CreateSolidBrush(COLORREF(0x00373230)); // RGB(48, 50, 55)
            let brush_btn_active = CreateSolidBrush(COLORREF(0x00453D3A)); // RGB(58, 61, 69)
            let brush_start_normal = CreateSolidBrush(COLORREF(0x002D2825)); // RGB(37, 40, 45)
            let brush_start_hover = CreateSolidBrush(COLORREF(0x00403733)); // RGB(51, 55, 64)
            let brush_accent = CreateSolidBrush(COLORREF(0x00D97706)); // Cyan/Blue Amber RGB(6, 119, 217)

            let start_menu = StartMenuWindow::create().ok();

            Self {
                hwnd,
                appbar: None,
                icon_cache: IconCache::new(),
                tracker: WindowTracker::new(200),
                event_rx,
                edge,
                thickness_px,
                hovered_target: None,
                button_layout: Vec::new(),
                font_main,
                font_bold,
                font_clock,
                brush_bg,
                brush_btn_normal,
                brush_btn_hover,
                brush_btn_active,
                brush_start_normal,
                brush_start_hover,
                brush_accent,
                start_menu,
                start_btn_rect: RECT::default(),
            }
        }
    }

    /// Process incoming events from the background tracking service.
    fn drain_events(&mut self) -> bool {
        let now_ms = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        let mut changed = false;
        while let Ok(event) = self.event_rx.try_recv() {
            if let WindowEvent::Destroyed(id) = event {
                self.icon_cache.remove(id);
            }
            self.tracker.apply_event(event, now_ms);
            changed = true;
        }
        changed
    }

    /// Calculate scaled taskbar height based on window DPI.
    fn calculate_scaled_thickness(&self) -> i32 {
        let dpi = unsafe { GetDpiForWindow(self.hwnd) };
        if dpi == 0 {
            self.thickness_px
        } else {
            (self.thickness_px * dpi as i32) / 96
        }
    }

    /// Render double-buffered taskbar UI via GDI.
    fn on_paint(&mut self) {
        unsafe {
            let mut ps = PAINTSTRUCT::default();
            let hdc = BeginPaint(self.hwnd, &mut ps);

            let mut client_rc = RECT::default();
            let _ = GetClientRect(self.hwnd, &mut client_rc);
            let width = client_rc.right - client_rc.left;
            let height = client_rc.bottom - client_rc.top;

            if width <= 0 || height <= 0 {
                let _ = EndPaint(self.hwnd, &ps);
                return;
            }

            // Create compatible backbuffer DC and bitmap
            let mem_dc = CreateCompatibleDC(Some(hdc));
            let mem_bmp = CreateCompatibleBitmap(hdc, width, height);
            let old_bmp = SelectObject(mem_dc, mem_bmp.into());

            // 1. Fill entire taskbar background
            let _ = FillRect(mem_dc, &client_rc, self.brush_bg);

            let mut button_rects: Vec<ButtonRect> = Vec::new();

            // 2. Render Start button (Left)
            let dpi = GetDpiForWindow(self.hwnd);
            let start_width = (105 * dpi as i32) / 96;
            let padding_y = (4 * dpi as i32) / 96;
            let start_rc = RECT {
                left: (6 * dpi as i32) / 96,
                top: padding_y,
                right: (6 * dpi as i32) / 96 + start_width,
                bottom: height - padding_y,
            };

            let is_start_hover = self.hovered_target == Some(ButtonTarget::Start);
            let start_brush = if is_start_hover {
                self.brush_start_hover
            } else {
                self.brush_start_normal
            };
            let _ = FillRect(mem_dc, &start_rc, start_brush);

            // Draw accent top/bottom bar on Start button when hovered
            if is_start_hover {
                let accent_line = RECT {
                    left: start_rc.left + 8,
                    right: start_rc.right - 8,
                    top: if self.edge == AppBarEdge::Top {
                        start_rc.bottom - 2
                    } else {
                        start_rc.top
                    },
                    bottom: if self.edge == AppBarEdge::Top {
                        start_rc.bottom
                    } else {
                        start_rc.top + 2
                    },
                };
                let _ = FillRect(mem_dc, &accent_line, self.brush_accent);
            }

            SetBkMode(mem_dc, TRANSPARENT);
            SetTextColor(mem_dc, COLORREF(0x00FFFFFF));
            let old_font = SelectObject(mem_dc, self.font_bold.into());

            let mut start_text_rc = start_rc;
            let mut start_label: Vec<u16> = "◆ Start".encode_utf16().collect();
            start_label.push(0);
            let _ = DrawTextW(
                mem_dc,
                &mut start_label,
                &mut start_text_rc,
                DT_CENTER | DT_VCENTER | DT_SINGLELINE,
            );

            self.start_btn_rect = start_rc;

            button_rects.push(ButtonRect {
                target: ButtonTarget::Start,
                rect: start_rc,
            });

            // 3. Render Running Application Buttons
            let mut cursor_x = start_rc.right + (8 * dpi as i32) / 96;
            let button_width = (175 * dpi as i32) / 96;
            let icon_size = (16 * dpi as i32) / 96;
            let taskbar_apps = self.tracker.taskbar_windows();

            let _ = SelectObject(mem_dc, self.font_main.into());

            // Reserve space for tray / clock on the right
            let clock_width = (130 * dpi as i32) / 96;
            let max_buttons_x = width - clock_width - (10 * dpi as i32) / 96;

            for app in taskbar_apps {
                if cursor_x + button_width > max_buttons_x {
                    break; // Prevent button overflow into clock
                }

                let btn_rc = RECT {
                    left: cursor_x,
                    top: padding_y,
                    right: cursor_x + button_width,
                    bottom: height - padding_y,
                };

                let is_hover = self.hovered_target == Some(ButtonTarget::Window(app.id));
                let is_fg = app.is_foreground;

                let btn_brush = if is_fg {
                    self.brush_btn_active
                } else if is_hover {
                    self.brush_btn_hover
                } else {
                    self.brush_btn_normal
                };

                let _ = FillRect(mem_dc, &btn_rc, btn_brush);

                // Highlight strip for active/foreground app
                if is_fg {
                    let accent_bar = RECT {
                        left: btn_rc.left + 4,
                        right: btn_rc.right - 4,
                        top: if self.edge == AppBarEdge::Top {
                            btn_rc.bottom - 3
                        } else {
                            btn_rc.top
                        },
                        bottom: if self.edge == AppBarEdge::Top {
                            btn_rc.bottom
                        } else {
                            btn_rc.top + 3
                        },
                    };
                    let _ = FillRect(mem_dc, &accent_bar, self.brush_accent);
                }

                // Draw App Icon
                let icon_y = btn_rc.top + (btn_rc.bottom - btn_rc.top - icon_size) / 2;
                let app_hwnd = HWND(app.id.0 as *mut _);
                if let Some(hicon) = self.icon_cache.get_or_extract(app_hwnd) {
                    let _ = DrawIconEx(
                        mem_dc,
                        btn_rc.left + 8,
                        icon_y,
                        hicon,
                        icon_size,
                        icon_size,
                        0,
                        None,
                        DI_NORMAL,
                    );
                } else {
                    // Fallback visual dot glyph
                    let dot_rc = RECT {
                        left: btn_rc.left + 10,
                        top: btn_rc.top + (btn_rc.bottom - btn_rc.top) / 2 - 4,
                        right: btn_rc.left + 18,
                        bottom: btn_rc.top + (btn_rc.bottom - btn_rc.top) / 2 + 4,
                    };
                    let _ = FillRect(mem_dc, &dot_rc, self.brush_accent);
                }

                // Draw App Title
                let text_color = if is_fg {
                    COLORREF(0x00FFFFFF)
                } else if app.is_minimized {
                    COLORREF(0x009E9590) // Dimmed for minimized
                } else {
                    COLORREF(0x00E0DCD8) // Crisp neutral white
                };
                SetTextColor(mem_dc, text_color);

                let mut text_rc = RECT {
                    left: btn_rc.left + 12 + icon_size,
                    top: btn_rc.top,
                    right: btn_rc.right - 6,
                    bottom: btn_rc.bottom,
                };

                let title_str = if app.title.is_empty() {
                    &app.process_name
                } else {
                    &app.title
                };
                let mut title_buf: Vec<u16> = title_str.encode_utf16().collect();
                title_buf.push(0);

                let _ = DrawTextW(
                    mem_dc,
                    &mut title_buf,
                    &mut text_rc,
                    DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS,
                );

                button_rects.push(ButtonRect {
                    target: ButtonTarget::Window(app.id),
                    rect: btn_rc,
                });

                cursor_x += button_width + (4 * dpi as i32) / 96;
            }

            // 4. Render System Clock & Date (Right)
            let _ = SelectObject(mem_dc, self.font_clock.into());
            let clock_width = (130 * dpi as i32) / 96;
            let clock_rc = RECT {
                left: width - clock_width - (6 * dpi as i32) / 96,
                top: padding_y,
                right: width - (6 * dpi as i32) / 96,
                bottom: height - padding_y,
            };

            let _ = FillRect(mem_dc, &clock_rc, self.brush_start_normal);
            SetTextColor(mem_dc, COLORREF(0x00FFFFFF));

            let now = chrono_or_fallback_time();
            let mut time_buf: Vec<u16> = now.encode_utf16().collect();
            time_buf.push(0);

            let mut clock_text_rc = clock_rc;
            let _ = DrawTextW(
                mem_dc,
                &mut time_buf,
                &mut clock_text_rc,
                DT_CENTER | DT_VCENTER | DT_SINGLELINE,
            );

            // Blit backbuffer to screen
            let _ = BitBlt(hdc, 0, 0, width, height, Some(mem_dc), 0, 0, SRCCOPY);

            // Cleanup GDI objects
            let _ = SelectObject(mem_dc, old_font);
            let _ = SelectObject(mem_dc, old_bmp);
            let _ = DeleteObject(mem_bmp.into());
            let _ = DeleteDC(mem_dc);
            let _ = EndPaint(self.hwnd, &ps);

            self.button_layout = button_rects;
        }
    }

    /// Click handler: activates or minimizes the clicked application window.
    fn on_lbutton_up(&mut self, x: i32, y: i32) {
        let pt = POINT { x, y };
        let mut clicked_btn = false;

        for btn in &self.button_layout {
            if pt_in_rect(&btn.rect, pt) {
                clicked_btn = true;
                match btn.target {
                    ButtonTarget::Start => {
                        info!("StartAllForward: Start button clicked");
                        if let Some(menu) = &mut self.start_menu {
                            if menu.is_visible() {
                                menu.hide();
                            } else if menu.was_recently_hidden() {
                                // Deactivation just occurred during this click gesture; keep closed
                            } else {
                                unsafe {
                                    let mut pt = POINT {
                                        x: self.start_btn_rect.left,
                                        y: self.start_btn_rect.top,
                                    };
                                    let _ = ClientToScreen(self.hwnd, &mut pt);
                                    let btn_w =
                                        self.start_btn_rect.right - self.start_btn_rect.left;
                                    let btn_h =
                                        self.start_btn_rect.bottom - self.start_btn_rect.top;
                                    let screen_rc = RECT {
                                        left: pt.x,
                                        top: pt.y,
                                        right: pt.x + btn_w,
                                        bottom: pt.y + btn_h,
                                    };
                                    menu.show_at(self.hwnd, self.edge, &screen_rc);
                                }
                            }
                        }
                    }
                    ButtonTarget::Window(id) => {
                        if let Some(menu) = &mut self.start_menu {
                            if menu.is_visible() {
                                menu.hide();
                            }
                        }
                        let target_hwnd = HWND(id.0 as *mut _);
                        activate_or_toggle_window(target_hwnd);
                    }
                }
                break;
            }
        }

        if !clicked_btn {
            // Clicked on empty taskbar area -> dismiss Start Menu if currently visible
            if let Some(menu) = &mut self.start_menu {
                if menu.is_visible() {
                    menu.hide();
                }
            }
        }
    }

    /// Hover tracking for visual responsiveness.
    fn on_mouse_move(&mut self, x: i32, y: i32) {
        let pt = POINT { x, y };
        let mut found = None;

        for btn in &self.button_layout {
            if pt_in_rect(&btn.rect, pt) {
                found = Some(btn.target);
                break;
            }
        }

        if found != self.hovered_target {
            self.hovered_target = found;
            unsafe {
                let _ = InvalidateRect(Some(self.hwnd), None, false);
            }
        }
    }

    /// Check if mouse has moved outside taskbar boundaries and clear hover.
    fn check_mouse_leave(&mut self) {
        if self.hovered_target.is_some() {
            unsafe {
                let mut pt = POINT::default();
                let _ = GetCursorPos(&mut pt);
                let mut rc = RECT::default();
                let _ = GetWindowRect(self.hwnd, &mut rc);
                if !pt_in_rect(&rc, pt) {
                    self.hovered_target = None;
                    let _ = InvalidateRect(Some(self.hwnd), None, false);
                }
            }
        }
    }
}

impl Drop for TaskbarState {
    fn drop(&mut self) {
        unsafe {
            let _ = DeleteObject(self.font_main.into());
            let _ = DeleteObject(self.font_bold.into());
            let _ = DeleteObject(self.font_clock.into());
            let _ = DeleteObject(self.brush_bg.into());
            let _ = DeleteObject(self.brush_btn_normal.into());
            let _ = DeleteObject(self.brush_btn_hover.into());
            let _ = DeleteObject(self.brush_btn_active.into());
            let _ = DeleteObject(self.brush_start_normal.into());
            let _ = DeleteObject(self.brush_start_hover.into());
            let _ = DeleteObject(self.brush_accent.into());
        }
        if let Some(menu) = &self.start_menu {
            menu.destroy();
        }
    }
}

/// Helper to activate or toggle window minimization.
fn activate_or_toggle_window(hwnd: HWND) {
    if hwnd.is_invalid() || !unsafe { IsWindow(Some(hwnd)).as_bool() } {
        warn!("Attempted to activate invalid or closed window {:?}", hwnd);
        return;
    }

    unsafe {
        let is_min = IsIconic(hwnd).as_bool();
        let fg = GetForegroundWindow();

        if is_min {
            let _ = ShowWindowAsync(hwnd, SW_RESTORE);
            let _ = SetForegroundWindow(hwnd);
        } else if fg == hwnd {
            // Already foreground -> minimize like standard Windows taskbar
            let _ = ShowWindowAsync(hwnd, SW_MINIMIZE);
        } else {
            let _ = SetForegroundWindow(hwnd);
        }
    }
}

fn pt_in_rect(rc: &RECT, pt: POINT) -> bool {
    pt.x >= rc.left && pt.x < rc.right && pt.y >= rc.top && pt.y < rc.bottom
}

fn chrono_or_fallback_time() -> String {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let secs_in_day = now % 86400;
    let hours = (secs_in_day / 3600) % 24;
    let minutes = (secs_in_day % 3600) / 60;
    let seconds = secs_in_day % 60;

    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}

/// Dedicated Win32 window class and message loop runner for the custom taskbar.
#[derive(Clone, Copy)]
pub struct CustomTaskbar {
    hwnd: isize,
}

impl CustomTaskbar {
    /// Create, register, and display the custom AppBar taskbar.
    pub fn spawn(
        edge: AppBarEdge,
        thickness_px: i32,
        event_rx: Receiver<WindowEvent>,
    ) -> Result<Self, String> {
        crate::desktop::try_attach_default_desktop();

        unsafe {
            let class_name = w!("StartAllForward_TaskbarClass");

            let wnd_class = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(taskbar_wnd_proc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: HINSTANCE::default(),
                hIcon: HICON::default(),
                hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
                hbrBackground: HBRUSH::default(),
                lpszMenuName: PCWSTR::null(),
                lpszClassName: class_name,
                hIconSm: HICON::default(),
            };

            let _ = RegisterClassExW(&wnd_class);

            let initial_thickness = if thickness_px > 0 {
                thickness_px
            } else {
                DEFAULT_TASKBAR_HEIGHT
            };

            let state = Box::new(TaskbarState::new(
                HWND::default(),
                edge,
                initial_thickness,
                event_rx,
            ));
            let state_ptr = Box::into_raw(state);

            // Borderless topmost toolwindow
            let create_res = CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
                class_name,
                w!("StartAllForward Taskbar"),
                WS_POPUP | WS_CLIPCHILDREN | WS_VISIBLE,
                0,
                0,
                100,
                initial_thickness,
                None,
                None,
                None,
                Some(state_ptr as *const _ as *const _),
            );

            let hwnd = match create_res {
                Ok(h) if !h.is_invalid() => h,
                _ => {
                    let _ = Box::from_raw(state_ptr);
                    return Err("Failed to create native taskbar window".to_string());
                }
            };

            info!(
                "Native taskbar window created successfully (HWND: {:?})",
                hwnd
            );

            Ok(Self {
                hwnd: hwnd.0 as isize,
            })
        }
    }

    /// Enter the dedicated Win32 message pump for the taskbar window.
    pub fn run_message_loop(&self) {
        unsafe {
            let mut msg = windows::Win32::UI::WindowsAndMessaging::MSG::default();
            loop {
                let ret = GetMessageW(&mut msg, None, 0, 0);
                if ret.0 <= 0 {
                    break;
                }
                let _ = windows::Win32::UI::WindowsAndMessaging::TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
        info!("Taskbar message loop exited.");
    }

    /// Post WM_CLOSE to gracefully close the taskbar window from another thread.
    pub fn request_close(&self) {
        let hwnd = HWND(self.hwnd as *mut _);
        if !hwnd.is_invalid() {
            unsafe {
                let _ = PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0));
            }
        }
    }

    /// Trigger immediate window closure.
    pub fn close(&self) {
        let hwnd = HWND(self.hwnd as *mut _);
        if !hwnd.is_invalid() {
            unsafe {
                let _ = DestroyWindow(hwnd);
            }
        }
    }

    /// Native HWND handle.
    pub fn hwnd(&self) -> HWND {
        HWND(self.hwnd as *mut _)
    }
}

unsafe extern "system" fn taskbar_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let state_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut TaskbarState;

    match msg {
        WM_CREATE => {
            let cs = &*(lparam.0 as *const CREATESTRUCTW);
            let state = &mut *(cs.lpCreateParams as *mut TaskbarState);
            state.hwnd = hwnd;
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, cs.lpCreateParams as isize);

            // Register AppBar
            let scaled_thickness = state.calculate_scaled_thickness();
            match AppBarManager::register(hwnd, state.edge, scaled_thickness) {
                Ok(manager) => {
                    state.appbar = Some(manager);
                }
                Err(e) => {
                    error!("AppBar registration error: {}", e);
                }
            }

            // Periodic refresh timer (4 FPS for clock/events)
            let _ = SetTimer(Some(hwnd), TIMER_REFRESH_ID, 250, None);
            LRESULT(0)
        }
        WM_APPBAR_CALLBACK => {
            if !state_ptr.is_null() {
                let state = &mut *state_ptr;
                if let Some(appbar) = &mut state.appbar {
                    appbar.on_appbar_callback(wparam.0);
                }
            }
            LRESULT(0)
        }
        WM_ACTIVATE => {
            if !state_ptr.is_null() {
                let state = &*state_ptr;
                if let Some(appbar) = &state.appbar {
                    appbar.on_activate();
                }
            }
            LRESULT(0)
        }
        WM_WINDOWPOSCHANGED => {
            if !state_ptr.is_null() {
                let state = &*state_ptr;
                if let Some(appbar) = &state.appbar {
                    appbar.on_window_pos_changed();
                }
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_DISPLAYCHANGE => {
            if !state_ptr.is_null() {
                let state = &mut *state_ptr;
                if let Some(appbar) = &mut state.appbar {
                    let _ = appbar.reposition();
                }
                let _ = InvalidateRect(Some(hwnd), None, false);
            }
            LRESULT(0)
        }
        WM_DPICHANGED => {
            if !state_ptr.is_null() {
                let state = &mut *state_ptr;
                let scaled = state.calculate_scaled_thickness();
                if let Some(appbar) = &mut state.appbar {
                    let _ = appbar.set_thickness(scaled);
                }
                let _ = InvalidateRect(Some(hwnd), None, false);
            }
            LRESULT(0)
        }
        WM_MOUSEACTIVATE => {
            // Crucial: Taskbar background must NOT activate or steal keyboard focus
            LRESULT(MA_NOACTIVATE as isize)
        }
        WM_NCHITTEST => {
            // Entire client area is interactive
            LRESULT(HTCLIENT as isize)
        }
        WM_PAINT => {
            if !state_ptr.is_null() {
                let state = &mut *state_ptr;
                state.on_paint();
            }
            LRESULT(0)
        }
        WM_ERASEBKGND => {
            // Suppress background erasing to eliminate flicker (double buffered in WM_PAINT)
            LRESULT(1)
        }
        WM_TIMER => {
            if wparam.0 == TIMER_REFRESH_ID && !state_ptr.is_null() {
                let state = &mut *state_ptr;
                let _ = state.drain_events();
                state.check_mouse_leave();
                // Redraw on timer tick (clock update)
                let _ = InvalidateRect(Some(hwnd), None, false);
            }
            LRESULT(0)
        }
        WM_MOUSEMOVE => {
            if !state_ptr.is_null() {
                let state = &mut *state_ptr;
                let x = (lparam.0 & 0xFFFF) as i16 as i32;
                let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as i32;
                state.on_mouse_move(x, y);
            }
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            if !state_ptr.is_null() {
                let state = &mut *state_ptr;
                let x = (lparam.0 & 0xFFFF) as i16 as i32;
                let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as i32;
                state.on_lbutton_up(x, y);
                let _ = InvalidateRect(Some(hwnd), None, false);
            }
            LRESULT(0)
        }
        WM_CLOSE => {
            unsafe {
                let _ = DestroyWindow(hwnd);
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            let _ = KillTimer(Some(hwnd), TIMER_REFRESH_ID);
            if !state_ptr.is_null() {
                let state = &mut *state_ptr;
                if let Some(mut appbar) = state.appbar.take() {
                    appbar.unregister();
                }
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                let _ = Box::from_raw(state_ptr);
            }
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
