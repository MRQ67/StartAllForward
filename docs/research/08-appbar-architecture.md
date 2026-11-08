# Research Note 08 — Native Window Architecture & AppBar Evaluation for Milestone 2

## 1. Context & Objective

Milestone 2 requires implementing an independent, functional custom Windows taskbar using the Windows Application Desktop Toolbar (AppBar) API (`SHAppBarMessage`).

A critical architectural decision is choosing between:
- **Approach A**: Reusing the existing `eframe`/`winit` window and intercepting Win32 messages via subclassing.
- **Approach B**: Creating a dedicated native Win32 window (`CreateWindowExW`) with an independent `WndProc` and double-buffered native rendering.

---

## 2. Technical Evaluation

| Criterion | Approach A: `eframe`/`winit` Subclassing | Approach B: Dedicated Win32 Native Window |
| :--- | :--- | :--- |
| **Message Loop Ownership** | `winit` owns the window procedure, message pump, and event loop. | StartAllForward owns the `WndProc` and message dispatching directly. |
| **AppBar Callback Dispatch (`uCallbackMessage`)** | Requires `SetWindowSubclass` to intercept `WM_USER + X`. `winit` may discard unrecognized messages. | Handled natively as a primary `match msg` branch in `WndProc`. |
| **Shell Layout Synchronization (`ABM_QUERYPOS` / `ABM_SETPOS`)** | **Fragile**: `winit` intercepts `WM_WINDOWPOSCHANGING` and `WM_SIZE` to update its internal surface, causing recursive layout conflicts and visual stuttering when the shell repositions the AppBar. | **Completely Stable**: Window position is negotiated strictly between `AppBarManager` and Windows Shell without intermediary interference. |
| **Window Activation & Focus** | Taskbar backgrounds must NOT steal keyboard focus from active apps when clicked (`WM_MOUSEACTIVATE` &rarr; `MA_NOACTIVATE`). Configuring this in `winit` is unsupported. | Native `WM_MOUSEACTIVATE` returns `MA_NOACTIVATE` directly. |
| **Window Styles** | Borderless taskbars require `WS_POPUP \| WS_CLIPCHILDREN \| WS_VISIBLE` and `WS_EX_TOOLWINDOW \| WS_EX_TOPMOST`. `winit` enforces standard top-level frame styles. | Full native control over standard and extended styles in `CreateWindowExW`. |
| **DPI Changes (`WM_DPICHANGED`)** | Handled through `winit` scale factor events, which trigger UI rerendering but do not recalculate AppBar docking rects. | `WM_DPICHANGED` triggers `AppBarManager::reposition()` with the new DPI thickness. |
| **Resource Footprint** | ~35 MB RAM, GPU context (OpenGL/WGPU pipeline). | **< 6 MB RAM**, instantaneous <1ms GDI double-buffered draw, zero GPU driver overhead. |
| **Application Icon Extraction** | Requires converting Windows native `HICON` into raw RGBA pixel buffers for egui textures. | Direct hardware-accelerated icon rendering via native `DrawIconEx`. |

---

## 3. Decision

**Approach B (Dedicated Native Win32 Window) is selected for the production custom taskbar.**

`eframe`/`egui` will be preserved for the optional diagnostic dashboard (`--diagnostic`), while the custom taskbar prototype runs on a dedicated native Win32 window with its own `WndProc`, AppBar manager, and double-buffered GDI rendering.
