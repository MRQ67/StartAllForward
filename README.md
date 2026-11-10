# StartAllForward

**StartAllForward** is an independent, free, and open-source Windows customization application and taskbar daemon built in **Rust**.

Inspired by the functionality of StartAllBack, StartAllForward is developed using a **clean-room, standalone user-mode architecture**:
- **Zero DLL injection** into `explorer.exe` or third-party processes.
- **Zero registry modifications** or Shell extension hijacking.
- **Strictly unprivileged**: Runs entirely as a standard user without administrator rights.
- **Safe coexistence**: Operates cleanly alongside the native Windows 11 shell and taskbar.

---

## Workspace Structure

The project is structured as a modular Cargo workspace:

- **`crates/startallforward-core`**: Platform-independent data models (`WindowId`, `WindowState`, `WindowEvent`, `AppItem`, `UserSettings`), the in-memory `WindowTracker`, and the indexed `AppSearchIndex`.
- **`crates/startallforward-windows`**: Native Windows API integration:
  - `AppBarManager`: Windows Application Desktop Toolbar (`SHAppBarMessage`) registration, workspace reservation, and docking.
  - `CustomTaskbar`: Dedicated borderless Win32 window with double-buffered GDI rendering, hover highlights, and click activation.
  - `StartMenuWindow`: Dedicated Win32 popup Start Menu with live search, keyboard navigation, view toggling, and auto-dismissal.
  - `app_discovery`: Known Folders traversal (`FOLDERID_Programs`, `FOLDERID_CommonPrograms`), safe `.lnk` COM resolution (`IShellLinkW`, `IPersistFile`), and packaged app enumeration.
  - `launcher`: Documented application launching via `ShellExecuteExW` and `IApplicationActivationManager`.
  - `settings_manager`: User configuration persistence under `%LOCALAPPDATA%\StartAllForward\settings.json`.
  - `WindowTrackingService`: WinEvent hooks (`SetWinEventHook`) and periodic reconciliation (`EnumWindows`).
  - `IconCache`: Safe extraction of running and installed application icons (`WM_GETICON`, `ExtractIconExW`, `SHGetFileInfoW`).
- **`crates/startallforward-ui`**: Diagnostic dashboard built with `eframe`/`egui` for inspecting window hierarchies and live event feeds.
- **`apps/startallforward`**: Application entry point supporting taskbar, diagnostic GUI, headless CLI, application listing, and launch test modes.

---

## Current Status & Product Roadmap

> 📖 **Full Product Roadmap**: See [**ROADMAP.md**](ROADMAP.md) for the unified phase-by-phase plan, technical dependencies, acceptance criteria, and architecture decisions.

### Milestone 1: Observational Window Tracking (Verified)
- Dedicated Win32 message pump thread running `SetWinEventHook` (`WINEVENT_OUTOFCONTEXT`).
- Periodic reconciliation loop recovering from dropped events.
- Shell taskbar eligibility filtering (styles, ownership, cloaking).
- Unit tests covering lifecycle, foreground switching, and stale handle pruning.

### Milestone 2: Custom AppBar Taskbar Prototype (Verified)
- Native borderless taskbar window docked to the **Top** or **Bottom** screen edge.
- Desktop workspace reservation via `SHAppBarMessage(ABM_NEW / ABM_QUERYPOS / ABM_SETPOS / ABM_REMOVE)`.
- Dynamic application buttons showing extracted icons, titles, and active foreground highlights.
- Interactive window activation (click to focus or minimize).
- StartAllForward Start button and live digital clock.
- Smooth DPI-aware scaling and clean unregistration on shutdown.

### Milestone 3: Native Start Menu & Application Launcher (Accepted)
- Dedicated native top-level popup Start Menu window styled with modern Windows 11 dark design.
- Full indexing of 155+ desktop shortcuts (`.lnk`) from User and Common Programs folders using COM `IShellLinkW` and `IPersistFile`.
- Verified integration with Windows packaged apps (Settings, Terminal, Calculator, Notepad) using `IApplicationActivationManager`.
- Real-time instant search with fuzzy/scored ranking (exact match, prefix, word boundary, and substring).
- Pinned applications and "All Apps" view tabs.
- Persistent configuration under `%LOCALAPPDATA%\StartAllForward\settings.json`.
- Full keyboard navigation (`Up`, `Down`, `Enter` to launch, `Escape` to close, `Tab` to switch views, instant search typing).
- Accurate placement above bottom taskbars or below top taskbars clamped within monitor work areas.
- Automatic dismissal on deactivation or application launch, with toggle click debounce.

### Upcoming Milestones (Roadmap Summary)
- **Phase 4 (Next)**: **Compact Taskbar Foundation** — Thickness options (28px–48px), density presets (Ultra-Compact, Compact, Default, Comfortable), and vertical screen space maximization for 1366×768 displays.
- **Phase 4A**: **Everything Search Feasibility** — Completed benchmarking of in-memory search and evaluated voidtools Everything IPC; deferred to keep core lightweight.
- **Phase 5**: **Conditional Async Search** — Target <100 µs tokenized in-memory search, extended System32 discovery, and opt-in Everything IPC.
- **Phase 6**: **Dedicated Wi-Fi Shortcut** — Direct taskbar trigger to access the native Windows network selection panel (activation candidates to be validated on Windows 11).
- **Phase 7**: **Classic Taskbar Behavior** — Never-combine / always-combine button grouping modes and dynamic text labels.
- **Phase 8**: **Polish & Reliability** — Unified settings interface, system theme detection, and multi-monitor secondary taskbars.

---

## Usage

### 1. Launch Custom Taskbar with Native Start Menu (Default: Top Edge)
```powershell
cargo run -p startallforward
```
*Click the `◆ Start` button on the taskbar to toggle the Start Menu.*

### 2. Dock to Bottom Edge
```powershell
cargo run -p startallforward -- --edge bottom
```

### 3. List All Indexed Applications (Diagnostic Test)
```powershell
cargo run -p startallforward -- --list-apps
```

### 4. Test Launch an Application via CLI
```powershell
cargo run -p startallforward -- --launch "calculator"
```

### 5. Automated Verification Run (Auto-close after N seconds)
```powershell
cargo run -p startallforward -- --edge top --duration 10
```

### 6. Launch Milestone 1 Diagnostic GUI (egui Dashboard)
```powershell
cargo run -p startallforward -- --diagnostic
```

### 7. Launch Milestone 1 CLI Observational Stream
```powershell
cargo run -p startallforward -- --cli --duration 10
```

---

## Verification & Testing

```powershell
# Run automated unit tests across all workspace crates (21 passed)
cargo test --workspace

# Run strict Clippy lints
cargo clippy --workspace --all-targets -- -D warnings

# Verify code formatting
cargo fmt --all -- --check
```

---

## Safety & Recovery Guarantee

StartAllForward never patches Windows binaries or tampers with `Shell_TrayWnd`. Closing the taskbar gracefully sends `ABM_REMOVE` to immediately restore the desktop workspace to full screen without requiring an Explorer restart.
