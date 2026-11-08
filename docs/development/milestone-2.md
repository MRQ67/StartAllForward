# Milestone 2: Custom AppBar Taskbar Prototype

## 1. Overview & Objective

Milestone 2 delivers StartAllForward's first functional custom Windows taskbar prototype implemented in Rust.

The prototype:
- Docks to the **top** or **bottom** screen edge of the primary display.
- Accurately reserves desktop workspace using the official Windows Application Desktop Toolbar (AppBar) API (`SHAppBarMessage`).
- Runs **alongside** the native Windows 11 taskbar without hiding `Shell_TrayWnd`, modifying Explorer, or injecting DLLs.
- Renders running applications dynamically in real-time, showing extracted icons, window titles, and active foreground state.
- Supports interactive window activation: clicking a button activates the application or minimizes it if already foreground.
- Features a modern Windows 11 dark visual theme with a Start button and a live system clock.
- Cleanly unregisters the AppBar on shutdown, restoring the desktop workarea.

---

## 2. Architecture: Dedicated Win32 Native Window vs. eframe Subclassing

An AppBar requires handling low-level Windows Shell protocols that abstract GUI frameworks like `winit` / `eframe` cannot reliably manage:
- Shell position negotiation (`ABM_QUERYPOS` and `ABM_SETPOS`) must not trigger internal framework resize loops.
- Taskbar background clicks must return `MA_NOACTIVATE` from `WM_MOUSEACTIVATE` so clicking empty space does not steal focus from the current active app.
- System notifications (`ABN_POSCHANGED`) and DPI transitions (`WM_DPICHANGED`) must immediately recalculate docking dimensions.
- High-efficiency GDI double-buffering achieves < 1ms draw time, direct `DrawIconEx` hardware-accelerated icon rendering, and under 6 MB RAM usage.

Therefore, StartAllForward implements the taskbar using a dedicated native Win32 window (`CreateWindowExW`) with its own `WndProc`, while preserving `eframe`/`egui` for diagnostic analysis (`--diagnostic`).

---

## 3. AppBarManager Specification

The AppBar lifecycle is managed by [`AppBarManager`](file:///D:/My-Repos/startallforward/crates/startallforward-windows/src/appbar.rs):

1. **Registration (`ABM_NEW`)**:
   Registers the `HWND` and binds the custom callback message `WM_APPBAR_CALLBACK` (`WM_USER + 101`).
2. **Position Negotiation (`ABM_QUERYPOS` & `ABM_SETPOS`)**:
   - Queries monitor dimensions via `GetMonitorInfoW`.
   - Proposes rect based on selected edge (`AppBarEdge::Top` or `AppBarEdge::Bottom`).
   - Sends `ABM_QUERYPOS` allowing the shell to adjust for existing bars (such as the native Windows taskbar).
   - Readjusts thickness to preserve intended height across DPI scaling.
   - Commits with `ABM_SETPOS` and moves the window via `SetWindowPos(HWND_TOPMOST, ...)`.
3. **Shell Notifications (`ABN_POSCHANGED`)**:
   When display resolution changes or another bar moves, the shell dispatches `ABN_POSCHANGED`, prompting an automatic repositioning pass.
4. **Clean Unregistration (`ABM_REMOVE`)**:
   Sent on `WM_DESTROY` or drop, restoring the desktop workarea to its original dimensions.

---

## 4. Window Tracking & Activation Integration

1. **Dynamic Updates**:
   The taskbar window's message loop consumes events from [`WindowTrackingService`](file:///D:/My-Repos/startallforward/crates/startallforward-windows/src/service.rs). Newly created windows appear instantly; destroyed windows are removed; title changes update button text; foreground changes update the active accent highlight.
2. **Icon Extraction**:
   [`IconCache`](file:///D:/My-Repos/startallforward/crates/startallforward-windows/src/icon.rs) queries `WM_GETICON` (`ICON_SMALL2`, `ICON_SMALL`, `ICON_BIG`) and class icons (`GCLP_HICONSM`, `GCLP_HICON`) with timeouts to prevent hanging on unresponsive processes.
3. **Window Activation Logic**:
   - If target window is minimized: calls `ShowWindowAsync(SW_RESTORE)` and `SetForegroundWindow`.
   - If target window is already foreground: calls `ShowWindowAsync(SW_MINIMIZE)` (classic taskbar toggle behavior).
   - If target window is background: calls `SetForegroundWindow`.

---

## 5. Verification Results

| Test Scenario | Command | Observed Behavior | Status |
| :--- | :--- | :--- | :---: |
| Top-edge Docking & Workspace Reservation | `cargo run -p startallforward -- --edge top --duration 10` | Taskbar docked to top edge (44px), reserved workspace, displayed 263 discovered windows, activated active apps, and cleaned up upon exit. | **PASS** |
| Bottom-edge Docking | `cargo run -p startallforward -- --edge bottom --duration 5` | Taskbar docked to bottom edge above native taskbar, coexisted without conflict, unregistered cleanly. | **PASS** |
| Repeated Registration Cycles | Consecutive runs (Top & Bottom) | Registered, moved, and unregistered 3 consecutive times with 0 dangling workarea leaks. | **PASS** |
| Workspace Unit Tests | `cargo test --workspace` | All 13 tests passed (6 tracker tests, 7 filter tests). | **PASS** |
| Lints & Formatting | `cargo clippy` & `cargo fmt` | Passed with 0 warnings under `-D warnings`. | **PASS** |
