# Milestone 1: Observational Window Tracking Architecture

## 1. Executive Summary

Milestone 1 establishes the foundational architecture for **StartAllForward**, an independent, open-source Windows customization daemon implemented in Rust. 

The implementation in this milestone is strictly **observational and diagnostic**. It does not dock to desktop edges, replace or hide the native Windows taskbar, intercept system hotkeys, modify the registry, or inject code into `explorer.exe` or any other process.

---

## 2. Workspace Organization

The project is structured as a Cargo workspace with strict separation between platform-agnostic models, Win32 platform integration, diagnostic UI presentation, and application entry points:

```
startallforward/
├── Cargo.toml                       # Workspace manifest & dependencies
├── crates/
│   ├── startallforward-core/         # Platform-independent models & tracker
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── models.rs            # WindowId, WindowState, WindowEvent, TrackerStats
│   │   │   └── tracker.rs           # In-memory WindowTracker, deduplication, log
│   │   └── tests/
│   │       └── tracker_tests.rs     # 100% pure unit test suite (0 OS dependencies)
│   ├── startallforward-windows/      # Win32 platform integration
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── api.rs               # Safe wrappers around Win32 window & process APIs
│   │       ├── filter.rs            # Taskbar eligibility rules (style & owner checks)
│   │       ├── hook.rs              # SetWinEventHook + dedicated message pump thread
│   │       ├── enumerator.rs        # EnumWindows desktop enumeration
│   │       └── service.rs           # WindowTrackingService coordinator
│   └── startallforward-ui/           # Observational UI components
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           └── app.rs               # DiagnosticApp (egui/eframe dashboard)
├── apps/
│   └── startallforward/             # Executable entry point
│       ├── Cargo.toml
│       └── src/
│           └── main.rs              # Dual-mode launcher (GUI & CLI headless modes)
└── docs/
    └── research/                    # Clean-room reverse engineering & architecture docs
```

---

## 3. Window Tracking & Win32 Integration

### A. Dedicated WinEvent Hook Thread
Win32 `SetWinEventHook` with `WINEVENT_OUTOFCONTEXT` requires a dedicated thread with an active Win32 message pump (`GetMessageW` / `DispatchMessageW`).
- The hook captures:
  - `EVENT_SYSTEM_FOREGROUND` (0x0003): Foreground window switches.
  - `EVENT_SYSTEM_MINIMIZESTART` (0x0016) / `EVENT_SYSTEM_MINIMIZEEND` (0x0017): Minimize/restore transitions.
  - `EVENT_OBJECT_CREATE` (0x8000) / `EVENT_OBJECT_DESTROY` (0x8001): Window creation and destruction.
  - `EVENT_OBJECT_SHOW` (0x8002) / `EVENT_OBJECT_HIDE` (0x8003): Visibility changes.
  - `EVENT_OBJECT_NAMECHANGE` (0x800C): Window title changes.
  - `EVENT_OBJECT_CLOAKED` (0x8017) / `EVENT_OBJECT_UNCLOAKED` (0x8018): Virtual desktop switching and modern app cloaking.
- Events are filtered to `OBJID_WINDOW` (0) and `CHILDID_SELF` (0) to eliminate event spam from child controls, carets, and menus.
- Clean shutdown is guaranteed via `PostThreadMessageW(thread_id, WM_QUIT, ...)` followed by `UnhookWinEvent`.

### B. Periodic Reconciliation
Because WinEvents can occasionally be dropped under extreme desktop load or when an application hangs during hook notification, `WindowTrackingService` runs a periodic reconciliation loop (default: every 2.5 seconds) using `EnumWindows`.
- Reconciles active windows against tracked state.
- Automatically prunes stale `HWND` handles for windows that terminated without notification.
- Updates titles, visibility, minimized state, and cloaking flags.

### C. Taskbar Eligibility Filtering
The classic Windows Shell rules are implemented in `filter.rs`:
- Window must be visible (`WS_VISIBLE`).
- Window must not be a child (`WS_CHILD`).
- Tool windows (`WS_EX_TOOLWINDOW`) are excluded unless overridden by `WS_EX_APPWINDOW`.
- Owned windows (`GW_OWNER != 0`) are excluded unless overridden by `WS_EX_APPWINDOW`.
- Cloaked windows (`DwmGetWindowAttribute(DWMWA_CLOAKED)`) are marked so they are not shown on active taskbars.

---

## 4. UI Framework Evaluation: `egui`/`eframe` vs. Production Taskbar

### Diagnostic Application (`egui`/`eframe`)
- `egui` and `eframe` provide instant, lightweight, cross-platform immediate-mode GUI rendering.
- For the Milestone 1 diagnostic dashboard, it allows real-time rendering of window lists, active state indicators, search filtering, and live event stream telemetry without UI lag.

### Production Taskbar Feasibility (`SHAppBarMessage`)
To act as a native Windows taskbar replacement in subsequent milestones:
1. **HWND Access**: In `winit` / `eframe`, the native `HWND` can be retrieved via `raw-window-handle` (`HasWindowHandle`).
2. **AppBar Registration**: The window must call `SHAppBarMessage(ABM_NEW, &mut appbar_data)`.
3. **Message Loop & Subclassing Requirements**:
   - An AppBar must receive custom callback messages (e.g. `ABN_POSCHANGED`, `ABN_STATECHANGE`) and handle `WM_ACTIVATE` and `WM_WINDOWPOSCHANGED` to negotiate docking dimensions with the Shell.
   - `eframe` abstracts the underlying window procedure. Intercepting these messages requires window subclassing via `SetWindowSubclass` / `DefSubclassProc` on the native HWND.
4. **Transparent DWM Backdrops (Mica / Acrylic)**:
   - Achieving standard Windows 11 taskbar aesthetics requires `DwmSetWindowAttribute` with `DWMWA_SYSTEMBACKDROP_TYPE` (Mica Alt or Acrylic), which requires borderless window styles (`WS_POPUP`) and DWM blur margins.
5. **Architectural Recommendation**:
   - Keep `egui`/`eframe` for the configuration/settings UI and diagnostic telemetry.
   - For the production taskbar bar itself, either subclass the `eframe`/`winit` HWND or create a lightweight, dedicated Win32 window (`CreateWindowExW`) with a DirectComposition / Direct2D or softbuffer surface.

---

## 5. Build and Verification Instructions

### Building the Workspace
```powershell
# Check workspace compile status
cargo check --workspace

# Run core unit tests (pure logic)
cargo test -p startallforward-core

# Build release binaries
cargo build --workspace --release
```

### Running the Diagnostic Prototype
```powershell
# Interactive GUI Mode (eframe dashboard)
cargo run -p startallforward

# Headless / CLI Observational Mode (prints table to stdout)
cargo run -p startallforward -- --cli

# Automated CLI verification pass for 10 seconds
cargo run -p startallforward -- --cli --duration 10
```

---

## 6. Environment Audit & Linker Discovery

### Active Toolchain
- `rustc 1.99.1 (58cf8f9c3 2026-10-13)`
- `cargo 1.99.1 (5f94df478 2026-08-27)`
- Active host target: `x86_64-pc-windows-msvc`

### Linker Finding
- The machine currently has the Visual Studio Installer installed, but the MSVC C++ Build Tools workload (`link.exe` and Windows SDK CRT libraries like `kernel32.lib`) is not installed.
- Because `windows 0.62` depends internally on `proc-macro2` and `quote` (which compile host `build.rs` binaries during build), a C++ linker is required to complete final linking.
- In accordance with Milestone 1 safety rules (*"Do not install dependencies or modify the system without approval"*), no installers or modifications were performed.
