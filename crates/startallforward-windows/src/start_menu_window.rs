use std::collections::HashMap;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use tracing::info;
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreateFontW, CreateSolidBrush,
    DeleteDC, DeleteObject, DrawTextW, EndPaint, FillRect, GetMonitorInfoW, InvalidateRect,
    MonitorFromWindow, SelectObject, SetBkMode, SetTextColor, DT_CENTER, DT_END_ELLIPSIS, DT_LEFT,
    DT_SINGLELINE, DT_VCENTER, FONT_CHARSET, FONT_CLIP_PRECISION, FONT_OUTPUT_PRECISION,
    FONT_QUALITY, FW_NORMAL, FW_SEMIBOLD, HBRUSH, HFONT, MONITORINFO, MONITOR_DEFAULTTOPRIMARY,
    PAINTSTRUCT, SRCCOPY, TRANSPARENT,
};
use windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES;
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::Shell::{
    ExtractIconExW, SHGetFileInfoW, SHFILEINFOW, SHGFI_ICON, SHGFI_SMALLICON,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyIcon, DestroyWindow, DrawIconEx, GetClientRect,
    GetWindowLongPtrW, IsWindow, IsWindowVisible, LoadCursorW, RegisterClassExW,
    SetForegroundWindow, SetWindowLongPtrW, SetWindowPos, ShowWindow, CS_HREDRAW, CS_VREDRAW,
    DI_NORMAL, GWLP_USERDATA, HICON, HWND_TOPMOST, IDC_ARROW, SWP_SHOWWINDOW, SW_HIDE, WA_INACTIVE,
    WM_ACTIVATE, WM_CHAR, WM_DESTROY, WM_ERASEBKGND, WM_KEYDOWN, WM_LBUTTONUP, WM_MOUSEMOVE,
    WM_MOUSEWHEEL, WM_PAINT, WNDCLASSEXW, WS_BORDER, WS_CLIPCHILDREN, WS_EX_TOOLWINDOW,
    WS_EX_TOPMOST, WS_POPUP,
};

use crate::app_discovery::discover_installed_apps;
use crate::appbar::AppBarEdge;
use crate::launcher::launch_app;
use crate::settings_manager::SettingsManager;
use startallforward_core::{AppItem, AppSearchIndex, StartMenuView, UserSettings};

const START_MENU_CLASS: PCWSTR = w!("StartAllForward_StartMenuClass");
const BASE_WIDTH: i32 = 440;
const BASE_HEIGHT: i32 = 580;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MenuClickTarget {
    TabPinned,
    TabAllApps,
    AppItem(usize),
}

struct MenuLayout {
    search_bar_rect: RECT,
    tab_pinned_rect: RECT,
    tab_all_rect: RECT,
    item_rects: Vec<(usize, RECT)>,
}

/// Internal state of the Start Menu popup window.
pub struct StartMenuState {
    hwnd: HWND,
    search_index: AppSearchIndex,
    settings_manager: SettingsManager,
    settings: UserSettings,
    search_query: String,
    current_view: StartMenuView,
    selected_index: usize,
    hovered_target: Option<MenuClickTarget>,
    scroll_offset: usize,
    icon_cache: HashMap<String, HICON>,
    font_title: HFONT,
    font_item: HFONT,
    font_sub: HFONT,
    brush_bg: HBRUSH,
    brush_card: HBRUSH,
    brush_hover: HBRUSH,
    brush_selected: HBRUSH,
    brush_accent: HBRUSH,
    brush_border: HBRUSH,
    layout: MenuLayout,
}

impl StartMenuState {
    pub fn new(hwnd: HWND) -> Self {
        let settings_manager = SettingsManager::new();
        let settings = settings_manager.load();
        let current_view = settings.default_view;

        let apps = discover_installed_apps();
        let mut search_index = AppSearchIndex::new(apps);
        search_index.update_pins(&settings.pinned_app_ids);

        unsafe {
            let dpi = GetDpiForWindow(hwnd);
            let font_h_title = -((14 * dpi as i32) / 96);
            let font_h_item = -((12 * dpi as i32) / 96);
            let font_h_sub = -((10 * dpi as i32) / 96);

            let font_title = CreateFontW(
                font_h_title,
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

            let font_item = CreateFontW(
                font_h_item,
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

            let font_sub = CreateFontW(
                font_h_sub,
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

            // Modern Dark Visual Palette
            let brush_bg = CreateSolidBrush(COLORREF(0x001E1C1C)); // RGB(28, 28, 30)
            let brush_card = CreateSolidBrush(COLORREF(0x002C2828)); // RGB(40, 40, 44)
            let brush_hover = CreateSolidBrush(COLORREF(0x003A3534)); // RGB(52, 53, 58)
            let brush_selected = CreateSolidBrush(COLORREF(0x004B4441)); // RGB(65, 68, 75)
            let brush_accent = CreateSolidBrush(COLORREF(0x00D97706)); // Cyan/Amber Accent
            let brush_border = CreateSolidBrush(COLORREF(0x003F3B3A)); // Subtle outline

            // Enable Windows 11 rounded corners if supported
            let round_corner_val = 2u32; // DWMWCP_ROUND
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_WINDOW_CORNER_PREFERENCE,
                &round_corner_val as *const _ as *const _,
                std::mem::size_of::<u32>() as u32,
            );

            Self {
                hwnd,
                search_index,
                settings_manager,
                settings,
                search_query: String::new(),
                current_view,
                selected_index: 0,
                hovered_target: None,
                scroll_offset: 0,
                icon_cache: HashMap::new(),
                font_title,
                font_item,
                font_sub,
                brush_bg,
                brush_card,
                brush_hover,
                brush_selected,
                brush_accent,
                brush_border,
                layout: MenuLayout {
                    search_bar_rect: RECT::default(),
                    tab_pinned_rect: RECT::default(),
                    tab_all_rect: RECT::default(),
                    item_rects: Vec::new(),
                },
            }
        }
    }

    /// Refresh pinned application status from settings.
    pub fn refresh_pins(&mut self) {
        self.settings = self.settings_manager.load();
        self.search_index.update_pins(&self.settings.pinned_app_ids);
    }

    /// Get current list of items to display based on search query or selected view.
    pub fn current_items(&self) -> Vec<AppItem> {
        let q = self.search_query.trim();
        if !q.is_empty() {
            self.search_index.search(q, 50)
        } else {
            match self.current_view {
                StartMenuView::Pinned => self.search_index.pinned_items(),
                StartMenuView::AllApps => self.search_index.all_alphabetical(),
            }
        }
    }

    /// Retrieve or extract an application icon.
    fn get_icon(&mut self, item: &AppItem) -> Option<HICON> {
        let key = if item.is_packaged {
            item.aumid.clone().unwrap_or_else(|| item.id.clone())
        } else {
            item.target_path.clone()
        };

        if let Some(&hicon) = self.icon_cache.get(&key) {
            return Some(hicon);
        }

        let icon_to_load = item.icon_path.as_deref().unwrap_or(&item.target_path);
        let path_wide: Vec<u16> = Path::new(icon_to_load)
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        unsafe {
            let mut hicon_small = HICON::default();
            let extracted = ExtractIconExW(
                PCWSTR(path_wide.as_ptr()),
                item.icon_index,
                None,
                Some(&mut hicon_small),
                1,
            );

            if extracted > 0 && !hicon_small.is_invalid() {
                self.icon_cache.insert(key, hicon_small);
                return Some(hicon_small);
            }

            // Fallback via SHGetFileInfoW
            let mut shfi = SHFILEINFOW::default();
            let res = SHGetFileInfoW(
                PCWSTR(path_wide.as_ptr()),
                FILE_FLAGS_AND_ATTRIBUTES(0),
                Some(&mut shfi),
                std::mem::size_of::<SHFILEINFOW>() as u32,
                SHGFI_ICON | SHGFI_SMALLICON,
            );

            if res != 0 && !shfi.hIcon.is_invalid() {
                self.icon_cache.insert(key, shfi.hIcon);
                return Some(shfi.hIcon);
            }
        }

        None
    }

    /// Double-buffered GDI paint handler.
    pub fn on_paint(&mut self) {
        let items = self.current_items();
        let is_search_active = !self.search_query.trim().is_empty();

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

            let mem_dc = CreateCompatibleDC(Some(hdc));
            let mem_bmp = CreateCompatibleBitmap(hdc, width, height);
            let old_bmp = SelectObject(mem_dc, mem_bmp.into());

            // 1. Fill Background
            let _ = FillRect(mem_dc, &client_rc, self.brush_bg);

            let dpi = GetDpiForWindow(self.hwnd);
            let pad = (12 * dpi as i32) / 96;

            // 2. Search Box (Top)
            let search_h = (38 * dpi as i32) / 96;
            let search_rc = RECT {
                left: pad,
                top: pad,
                right: width - pad,
                bottom: pad + search_h,
            };
            let _ = FillRect(mem_dc, &search_rc, self.brush_card);

            SetBkMode(mem_dc, TRANSPARENT);
            let old_font = SelectObject(mem_dc, self.font_item.into());

            // Draw Search Icon and Text
            let mut search_icon_rc = RECT {
                left: search_rc.left + (8 * dpi as i32) / 96,
                top: search_rc.top,
                right: search_rc.left + (28 * dpi as i32) / 96,
                bottom: search_rc.bottom,
            };
            SetTextColor(mem_dc, COLORREF(0x009E9590));
            let mut search_icon_str: Vec<u16> =
                "🔍".encode_utf16().chain(std::iter::once(0)).collect();
            let _ = DrawTextW(
                mem_dc,
                &mut search_icon_str,
                &mut search_icon_rc,
                DT_CENTER | DT_VCENTER | DT_SINGLELINE,
            );

            let mut search_text_rc = RECT {
                left: search_icon_rc.right + (6 * dpi as i32) / 96,
                top: search_rc.top,
                right: search_rc.right - (8 * dpi as i32) / 96,
                bottom: search_rc.bottom,
            };

            if self.search_query.is_empty() {
                SetTextColor(mem_dc, COLORREF(0x00807874));
                let mut placeholder: Vec<u16> = "Type here to search apps..."
                    .encode_utf16()
                    .chain(std::iter::once(0))
                    .collect();
                let _ = DrawTextW(
                    mem_dc,
                    &mut placeholder,
                    &mut search_text_rc,
                    DT_LEFT | DT_VCENTER | DT_SINGLELINE,
                );
            } else {
                SetTextColor(mem_dc, COLORREF(0x00FFFFFF));
                let query_with_cursor = format!("{}|", self.search_query);
                let mut query_buf: Vec<u16> = query_with_cursor
                    .encode_utf16()
                    .chain(std::iter::once(0))
                    .collect();
                let _ = DrawTextW(
                    mem_dc,
                    &mut query_buf,
                    &mut search_text_rc,
                    DT_LEFT | DT_VCENTER | DT_SINGLELINE,
                );
            }

            self.layout.search_bar_rect = search_rc;

            // 3. View Switcher Tabs (Only if not in active search mode)
            let mut current_y = search_rc.bottom + (10 * dpi as i32) / 96;
            let tab_h = (28 * dpi as i32) / 96;
            let tab_w = (90 * dpi as i32) / 96;

            if !is_search_active {
                let mut tab_pinned_rc = RECT {
                    left: pad,
                    top: current_y,
                    right: pad + tab_w,
                    bottom: current_y + tab_h,
                };

                let mut tab_all_rc = RECT {
                    left: tab_pinned_rc.right + (6 * dpi as i32) / 96,
                    top: current_y,
                    right: tab_pinned_rc.right + (6 * dpi as i32) / 96 + tab_w,
                    bottom: current_y + tab_h,
                };

                // Pinned Tab
                let is_pinned_active = self.current_view == StartMenuView::Pinned;
                let pinned_brush = if is_pinned_active {
                    self.brush_selected
                } else if self.hovered_target == Some(MenuClickTarget::TabPinned) {
                    self.brush_hover
                } else {
                    self.brush_bg
                };
                let _ = FillRect(mem_dc, &tab_pinned_rc, pinned_brush);

                SetTextColor(
                    mem_dc,
                    if is_pinned_active {
                        COLORREF(0x00FFFFFF)
                    } else {
                        COLORREF(0x00A09B98)
                    },
                );
                let mut pinned_str: Vec<u16> = "★ Pinned"
                    .encode_utf16()
                    .chain(std::iter::once(0))
                    .collect();
                let _ = DrawTextW(
                    mem_dc,
                    &mut pinned_str,
                    &mut tab_pinned_rc,
                    DT_CENTER | DT_VCENTER | DT_SINGLELINE,
                );

                // All Apps Tab
                let is_all_active = self.current_view == StartMenuView::AllApps;
                let all_brush = if is_all_active {
                    self.brush_selected
                } else if self.hovered_target == Some(MenuClickTarget::TabAllApps) {
                    self.brush_hover
                } else {
                    self.brush_bg
                };
                let _ = FillRect(mem_dc, &tab_all_rc, all_brush);

                SetTextColor(
                    mem_dc,
                    if is_all_active {
                        COLORREF(0x00FFFFFF)
                    } else {
                        COLORREF(0x00A09B98)
                    },
                );
                let mut all_str: Vec<u16> = "☰ All Apps"
                    .encode_utf16()
                    .chain(std::iter::once(0))
                    .collect();
                let _ = DrawTextW(
                    mem_dc,
                    &mut all_str,
                    &mut tab_all_rc,
                    DT_CENTER | DT_VCENTER | DT_SINGLELINE,
                );

                self.layout.tab_pinned_rect = tab_pinned_rc;
                self.layout.tab_all_rect = tab_all_rc;

                current_y += tab_h + (8 * dpi as i32) / 96;
            } else {
                // In search mode, show search header label
                SetTextColor(mem_dc, COLORREF(0x00B0ABA7));
                let _ = SelectObject(mem_dc, self.font_sub.into());
                let mut header_rc = RECT {
                    left: pad + 4,
                    top: current_y,
                    right: width - pad,
                    bottom: current_y + (18 * dpi as i32) / 96,
                };
                let mut search_header: Vec<u16> = format!("Best matches ({})", items.len())
                    .encode_utf16()
                    .chain(std::iter::once(0))
                    .collect();
                let _ = DrawTextW(
                    mem_dc,
                    &mut search_header,
                    &mut header_rc,
                    DT_LEFT | DT_VCENTER | DT_SINGLELINE,
                );
                current_y += (22 * dpi as i32) / 96;
            }

            // 4. Render Applications List
            let item_h = (42 * dpi as i32) / 96;
            let icon_size = (24 * dpi as i32) / 96;
            let footer_h = (36 * dpi as i32) / 96;
            let max_list_bottom = height - footer_h;

            let mut new_item_rects = Vec::new();
            let mut list_y = current_y;

            if items.is_empty() {
                SetTextColor(mem_dc, COLORREF(0x00807874));
                let _ = SelectObject(mem_dc, self.font_item.into());
                let mut empty_rc = RECT {
                    left: pad,
                    top: list_y + 30,
                    right: width - pad,
                    bottom: list_y + 80,
                };
                let mut empty_msg: Vec<u16> = "No applications found"
                    .encode_utf16()
                    .chain(std::iter::once(0))
                    .collect();
                let _ = DrawTextW(
                    mem_dc,
                    &mut empty_msg,
                    &mut empty_rc,
                    DT_CENTER | DT_VCENTER | DT_SINGLELINE,
                );
            } else {
                let visible_items = items.iter().enumerate().skip(self.scroll_offset);

                for (idx, item) in visible_items {
                    if list_y + item_h > max_list_bottom {
                        break;
                    }

                    let item_rc = RECT {
                        left: pad,
                        top: list_y,
                        right: width - pad,
                        bottom: list_y + item_h,
                    };

                    let is_selected = idx == self.selected_index;
                    let is_hovered = self.hovered_target == Some(MenuClickTarget::AppItem(idx));

                    let item_brush = if is_selected {
                        self.brush_selected
                    } else if is_hovered {
                        self.brush_hover
                    } else {
                        self.brush_bg
                    };

                    let _ = FillRect(mem_dc, &item_rc, item_brush);

                    // Selected accent bar on left edge
                    if is_selected {
                        let bar_rc = RECT {
                            left: item_rc.left,
                            top: item_rc.top + 4,
                            right: item_rc.left + 3,
                            bottom: item_rc.bottom - 4,
                        };
                        let _ = FillRect(mem_dc, &bar_rc, self.brush_accent);
                    }

                    // Draw Icon
                    let icon_y = item_rc.top + (item_rc.bottom - item_rc.top - icon_size) / 2;
                    let icon_x = item_rc.left + (10 * dpi as i32) / 96;

                    if let Some(hicon) = self.get_icon(item) {
                        let _ = DrawIconEx(
                            mem_dc, icon_x, icon_y, hicon, icon_size, icon_size, 0, None, DI_NORMAL,
                        );
                    } else {
                        // Fallback square
                        let sq_rc = RECT {
                            left: icon_x,
                            top: icon_y,
                            right: icon_x + icon_size,
                            bottom: icon_y + icon_size,
                        };
                        let _ = FillRect(mem_dc, &sq_rc, self.brush_accent);
                    }

                    // Draw App Title
                    let _ = SelectObject(mem_dc, self.font_item.into());
                    SetTextColor(
                        mem_dc,
                        if is_selected {
                            COLORREF(0x00FFFFFF)
                        } else {
                            COLORREF(0x00E4DFDC)
                        },
                    );

                    let text_x = icon_x + icon_size + (10 * dpi as i32) / 96;
                    let mut title_rc = RECT {
                        left: text_x,
                        top: item_rc.top + 3,
                        right: item_rc.right - 10,
                        bottom: item_rc.top + (item_h * 55) / 100,
                    };

                    let mut title_buf: Vec<u16> =
                        item.name.encode_utf16().chain(std::iter::once(0)).collect();
                    let _ = DrawTextW(
                        mem_dc,
                        &mut title_buf,
                        &mut title_rc,
                        DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS,
                    );

                    // Draw Subtitle / Category
                    let _ = SelectObject(mem_dc, self.font_sub.into());
                    SetTextColor(mem_dc, COLORREF(0x008E8884));
                    let mut sub_rc = RECT {
                        left: text_x,
                        top: title_rc.bottom - 2,
                        right: item_rc.right - 10,
                        bottom: item_rc.bottom - 3,
                    };

                    let sub_text = if item.is_packaged {
                        "Windows App"
                    } else if item.target_path.is_empty() {
                        "Desktop Application"
                    } else {
                        &item.target_path
                    };

                    let mut sub_buf: Vec<u16> =
                        sub_text.encode_utf16().chain(std::iter::once(0)).collect();
                    let _ = DrawTextW(
                        mem_dc,
                        &mut sub_buf,
                        &mut sub_rc,
                        DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS,
                    );

                    new_item_rects.push((idx, item_rc));
                    list_y += item_h + (2 * dpi as i32) / 96;
                }
            }

            self.layout.item_rects = new_item_rects;

            // 5. Footer Strip (Bottom)
            let footer_rc = RECT {
                left: 0,
                top: height - footer_h,
                right: width,
                bottom: height,
            };
            let _ = FillRect(mem_dc, &footer_rc, self.brush_card);

            let _ = SelectObject(mem_dc, self.font_sub.into());
            SetTextColor(mem_dc, COLORREF(0x00908B87));

            let mut hint_rc = RECT {
                left: pad,
                top: footer_rc.top,
                right: width - pad,
                bottom: footer_rc.bottom,
            };
            let mut hint_str: Vec<u16> = "StartAllForward • Enter: Launch • Esc: Close • Tab: View"
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();
            let _ = DrawTextW(
                mem_dc,
                &mut hint_str,
                &mut hint_rc,
                DT_LEFT | DT_VCENTER | DT_SINGLELINE,
            );

            // Copy to screen
            let _ = BitBlt(hdc, 0, 0, width, height, Some(mem_dc), 0, 0, SRCCOPY);

            // Cleanup GDI objects
            let _ = SelectObject(mem_dc, old_font);
            let _ = SelectObject(mem_dc, old_bmp);
            let _ = DeleteObject(mem_bmp.into());
            let _ = DeleteDC(mem_dc);
            let _ = EndPaint(self.hwnd, &ps);
        }
    }

    /// Launch selected application and hide menu.
    pub fn launch_selected(&mut self) {
        let items = self.current_items();
        if let Some(item) = items.get(self.selected_index) {
            info!("Launching from Start Menu: {}", item.name);
            let _ = launch_app(item);
            self.hide();
        }
    }

    /// Hide the Start Menu popup.
    pub fn hide(&mut self) {
        unsafe {
            let _ = ShowWindow(self.hwnd, SW_HIDE);
        }
        self.search_query.clear();
        self.selected_index = 0;
        self.scroll_offset = 0;
        self.hovered_target = None;
    }

    /// Show and position the Start Menu popup adjacent to the Start button and taskbar.
    pub fn show_at(&mut self, taskbar_hwnd: HWND, edge: AppBarEdge, start_btn_screen_rc: &RECT) {
        self.refresh_pins();
        self.search_query.clear();
        self.selected_index = 0;
        self.scroll_offset = 0;

        unsafe {
            let dpi = GetDpiForWindow(self.hwnd);
            let width = (BASE_WIDTH * dpi as i32) / 96;
            let height = (BASE_HEIGHT * dpi as i32) / 96;

            // Query monitor workarea to clamp window properly
            let hmon = MonitorFromWindow(taskbar_hwnd, MONITOR_DEFAULTTOPRIMARY);
            let mut mi = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            let _ = GetMonitorInfoW(hmon, &mut mi);
            let work_rc = mi.rcWork;

            let gap = (8 * dpi as i32) / 96;
            let top = if edge == AppBarEdge::Bottom {
                start_btn_screen_rc.top - height - gap
            } else {
                start_btn_screen_rc.bottom + gap
            };

            let left = start_btn_screen_rc.left;

            // Clamp inside workarea
            let clamped_left = left.max(work_rc.left).min(work_rc.right - width);
            let clamped_top = top.max(work_rc.top).min(work_rc.bottom - height);

            let _ = SetWindowPos(
                self.hwnd,
                Some(HWND_TOPMOST),
                clamped_left,
                clamped_top,
                width,
                height,
                SWP_SHOWWINDOW,
            );

            let _ = SetForegroundWindow(self.hwnd);
            let _ = InvalidateRect(Some(self.hwnd), None, false);
        }
    }

    pub fn is_visible(&self) -> bool {
        unsafe { IsWindowVisible(self.hwnd).as_bool() }
    }

    pub fn on_lbutton_up(&mut self, x: i32, y: i32) {
        let pt = POINT { x, y };

        if pt_in_rect(&self.layout.tab_pinned_rect, pt) {
            self.current_view = StartMenuView::Pinned;
            self.selected_index = 0;
            self.scroll_offset = 0;
            unsafe {
                let _ = InvalidateRect(Some(self.hwnd), None, false);
            }
            return;
        }

        if pt_in_rect(&self.layout.tab_all_rect, pt) {
            self.current_view = StartMenuView::AllApps;
            self.selected_index = 0;
            self.scroll_offset = 0;
            unsafe {
                let _ = InvalidateRect(Some(self.hwnd), None, false);
            }
            return;
        }

        for &(idx, ref rc) in &self.layout.item_rects {
            if pt_in_rect(rc, pt) {
                self.selected_index = idx;
                self.launch_selected();
                return;
            }
        }
    }

    pub fn on_mouse_move(&mut self, x: i32, y: i32) {
        let pt = POINT { x, y };
        let mut target = None;

        if pt_in_rect(&self.layout.tab_pinned_rect, pt) {
            target = Some(MenuClickTarget::TabPinned);
        } else if pt_in_rect(&self.layout.tab_all_rect, pt) {
            target = Some(MenuClickTarget::TabAllApps);
        } else {
            for &(idx, ref rc) in &self.layout.item_rects {
                if pt_in_rect(rc, pt) {
                    target = Some(MenuClickTarget::AppItem(idx));
                    break;
                }
            }
        }

        if target != self.hovered_target {
            self.hovered_target = target;
            unsafe {
                let _ = InvalidateRect(Some(self.hwnd), None, false);
            }
        }
    }

    pub fn on_key_down(&mut self, vk: WPARAM) {
        let items_len = self.current_items().len();
        match vk.0 as u16 {
            27 => {
                // VK_ESCAPE
                self.hide();
            }
            13 => {
                // VK_RETURN
                self.launch_selected();
            }
            38 => {
                // VK_UP
                self.selected_index = self.selected_index.saturating_sub(1);
                if self.selected_index < self.scroll_offset {
                    self.scroll_offset = self.selected_index;
                }
                unsafe {
                    let _ = InvalidateRect(Some(self.hwnd), None, false);
                }
            }
            40 => {
                // VK_DOWN
                if items_len > 0 {
                    self.selected_index = (self.selected_index + 1).min(items_len - 1);
                    if self.selected_index >= self.scroll_offset + 9 {
                        self.scroll_offset = self.selected_index.saturating_sub(8);
                    }
                    unsafe {
                        let _ = InvalidateRect(Some(self.hwnd), None, false);
                    }
                }
            }
            9 => {
                // VK_TAB
                self.current_view = match self.current_view {
                    StartMenuView::Pinned => StartMenuView::AllApps,
                    StartMenuView::AllApps => StartMenuView::Pinned,
                };
                self.selected_index = 0;
                self.scroll_offset = 0;
                unsafe {
                    let _ = InvalidateRect(Some(self.hwnd), None, false);
                }
            }
            _ => {}
        }
    }

    pub fn on_char(&mut self, ch: WPARAM) {
        let val = ch.0 as u32;
        if val == 8 {
            // Backspace
            self.search_query.pop();
            self.selected_index = 0;
            self.scroll_offset = 0;
            unsafe {
                let _ = InvalidateRect(Some(self.hwnd), None, false);
            }
        } else if val == 27 || val == 13 || val == 9 {
            // Handled in key_down
        } else if let Some(c) = char::from_u32(val) {
            if !c.is_control() {
                self.search_query.push(c);
                self.selected_index = 0;
                self.scroll_offset = 0;
                unsafe {
                    let _ = InvalidateRect(Some(self.hwnd), None, false);
                }
            }
        }
    }

    pub fn on_mouse_wheel(&mut self, delta: i16) {
        let items_len = self.current_items().len();
        if delta > 0 {
            self.scroll_offset = self.scroll_offset.saturating_sub(2);
        } else if items_len > 8 {
            self.scroll_offset = (self.scroll_offset + 2).min(items_len - 8);
        }
        unsafe {
            let _ = InvalidateRect(Some(self.hwnd), None, false);
        }
    }
}

impl Drop for StartMenuState {
    fn drop(&mut self) {
        unsafe {
            for (_, hicon) in self.icon_cache.drain() {
                let _ = DestroyIcon(hicon);
            }
            let _ = DeleteObject(self.font_title.into());
            let _ = DeleteObject(self.font_item.into());
            let _ = DeleteObject(self.font_sub.into());
            let _ = DeleteObject(self.brush_bg.into());
            let _ = DeleteObject(self.brush_card.into());
            let _ = DeleteObject(self.brush_hover.into());
            let _ = DeleteObject(self.brush_selected.into());
            let _ = DeleteObject(self.brush_accent.into());
            let _ = DeleteObject(self.brush_border.into());
        }
    }
}

fn pt_in_rect(rc: &RECT, pt: POINT) -> bool {
    pt.x >= rc.left && pt.x < rc.right && pt.y >= rc.top && pt.y < rc.bottom
}

/// Start Menu Win32 Window Manager wrapper.
pub struct StartMenuWindow {
    hwnd: isize,
}

impl StartMenuWindow {
    /// Create and register the native Start Menu popup window.
    pub fn create() -> Result<Self, String> {
        unsafe {
            let instance: HINSTANCE = windows::Win32::System::LibraryLoader::GetModuleHandleW(None)
                .map(|h| HINSTANCE(h.0))
                .unwrap_or_default();

            let cursor = LoadCursorW(None, IDC_ARROW).map_err(|e| e.to_string())?;

            let wc = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(start_menu_wnd_proc),
                hInstance: instance,
                hCursor: cursor,
                lpszClassName: START_MENU_CLASS,
                ..Default::default()
            };

            let _ = RegisterClassExW(&wc);

            let hwnd = CreateWindowExW(
                WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
                START_MENU_CLASS,
                w!("StartAllForward Start Menu"),
                WS_POPUP | WS_CLIPCHILDREN | WS_BORDER,
                0,
                0,
                BASE_WIDTH,
                BASE_HEIGHT,
                None,
                None,
                Some(instance),
                None,
            )
            .map_err(|e| format!("Failed to create Start Menu window: {}", e))?;

            let state = Box::new(StartMenuState::new(hwnd));
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(state) as isize);

            Ok(Self {
                hwnd: hwnd.0 as isize,
            })
        }
    }

    pub fn hwnd(&self) -> HWND {
        HWND(self.hwnd as *mut _)
    }

    pub fn is_visible(&self) -> bool {
        let hwnd = self.hwnd();
        if hwnd.is_invalid() || !unsafe { IsWindow(Some(hwnd)).as_bool() } {
            return false;
        }
        unsafe {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut StartMenuState;
            if !ptr.is_null() {
                (*ptr).is_visible()
            } else {
                false
            }
        }
    }

    pub fn hide(&self) {
        let hwnd = self.hwnd();
        if !hwnd.is_invalid() && unsafe { IsWindow(Some(hwnd)).as_bool() } {
            unsafe {
                let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut StartMenuState;
                if !ptr.is_null() {
                    (*ptr).hide();
                }
            }
        }
    }

    pub fn show_at(&self, taskbar_hwnd: HWND, edge: AppBarEdge, start_btn_screen_rc: &RECT) {
        let hwnd = self.hwnd();
        if !hwnd.is_invalid() && unsafe { IsWindow(Some(hwnd)).as_bool() } {
            unsafe {
                let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut StartMenuState;
                if !ptr.is_null() {
                    (*ptr).show_at(taskbar_hwnd, edge, start_btn_screen_rc);
                }
            }
        }
    }

    pub fn destroy(&self) {
        let hwnd = self.hwnd();
        if !hwnd.is_invalid() && unsafe { IsWindow(Some(hwnd)).as_bool() } {
            unsafe {
                let _ = DestroyWindow(hwnd);
            }
        }
    }
}

/// Window Procedure for Start Menu.
unsafe extern "system" fn start_menu_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let state_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut StartMenuState;

    match msg {
        WM_PAINT => {
            if !state_ptr.is_null() {
                (*state_ptr).on_paint();
                return LRESULT(0);
            }
        }
        WM_ERASEBKGND => {
            return LRESULT(1);
        }
        WM_ACTIVATE => {
            // Dismiss menu if deactivated
            let active_flag = (wparam.0 & 0xFFFF) as u32;
            if active_flag == WA_INACTIVE && !state_ptr.is_null() {
                (*state_ptr).hide();
            }
            return LRESULT(0);
        }
        WM_KEYDOWN => {
            if !state_ptr.is_null() {
                (*state_ptr).on_key_down(wparam);
                return LRESULT(0);
            }
        }
        WM_CHAR => {
            if !state_ptr.is_null() {
                (*state_ptr).on_char(wparam);
                return LRESULT(0);
            }
        }
        WM_MOUSEWHEEL => {
            if !state_ptr.is_null() {
                let delta = ((wparam.0 >> 16) & 0xFFFF) as i16;
                (*state_ptr).on_mouse_wheel(delta);
                return LRESULT(0);
            }
        }
        WM_LBUTTONUP => {
            if !state_ptr.is_null() {
                let x = (lparam.0 & 0xFFFF) as i16 as i32;
                let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as i32;
                (*state_ptr).on_lbutton_up(x, y);
                return LRESULT(0);
            }
        }
        WM_MOUSEMOVE => {
            if !state_ptr.is_null() {
                let x = (lparam.0 & 0xFFFF) as i16 as i32;
                let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as i32;
                (*state_ptr).on_mouse_move(x, y);
                return LRESULT(0);
            }
        }
        WM_DESTROY => {
            if !state_ptr.is_null() {
                let _ = Box::from_raw(state_ptr);
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            }
            return LRESULT(0);
        }
        _ => {}
    }

    DefWindowProcW(hwnd, msg, wparam, lparam)
}
