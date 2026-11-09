# Milestone 3: Native Start Menu & Application Launcher

## 1. Overview & Objective

Milestone 3 implements StartAllForward's independent native Start Menu and application launcher in Rust.

The Start Menu:
- Provides a clean, modern Windows 11-inspired launcher with classic Start Menu usability.
- Indexes installed desktop applications from both the user's and common Start Menu Programs directories.
- Safely resolves `.lnk` shortcuts, arguments, working directories, and icons via Windows Shell COM interfaces (`IShellLinkW`, `IPersistFile`).
- Supports packaged applications (UWP/MSIX) via Application User Model IDs (AUMIDs) and `IApplicationActivationManager`.
- Features real-time search filtering with instant typing, keyboard navigation (`Up`, `Down`, `Enter` to launch, `Escape` to close), and mouse selection.
- Provides "Pinned" and "All Apps" views.
- Persists user pins and launcher settings in `%LOCALAPPDATA%\StartAllForward\settings.json`.
- Accurately positions the popup window relative to the taskbar (above a bottom-docked taskbar or below a top-docked taskbar), respecting the usable monitor work area.
- Toggles on Start button click (`◆ Start`) and automatically dismisses on outside click, window deactivation, or application launch.
- Operates with zero DLL injection, zero registry modification, and zero administrative elevation.

---

## 2. Architecture & Design Decisions

### 2.1 Native Popup Window vs. Embedded Child
The Start Menu is implemented as a dedicated top-level Win32 popup window (`CreateWindowExW` with `WS_POPUP | WS_CLIPCHILDREN | WS_BORDER` and `WS_EX_TOOLWINDOW`):
- **Why a separate top-level window**: An AppBar is constrained by Shell workspace layout; embedding a 600px tall flyout inside a 44px AppBar would cause clipping. A separate top-level popup can float above desktop windows and dock directly adjacent to the AppBar.
- **Deactivation Dismissal**: The window intercepts `WM_ACTIVATE`. When `LOWORD(wparam) == WA_INACTIVE`, it automatically dismisses (`ShowWindowAsync(hwnd, SW_HIDE)`), providing standard Windows popup behavior.
- **Keyboard Focus**: Unlike the taskbar (which returns `MA_NOACTIVATE` to avoid stealing focus from running apps), opening the Start Menu transfers focus to the menu window so the user can immediately type to search or navigate with arrow keys.

### 2.2 Application Discovery Pipeline
1. **Known Folders Resolution**:
   - `FOLDERID_Programs`: User-specific `%APPDATA%\Microsoft\Windows\Start Menu\Programs`.
   - `FOLDERID_CommonPrograms`: Machine-wide `%ProgramData%\Microsoft\Windows\Start Menu\Programs`.
2. **Recursive Traversal & Filtering**:
   - Recursively walks both trees for `.lnk` files.
   - Ignores uninstallers (`*uninstall*`, `*uninst*`, `*setup*`), help/readmes (`*.txt`, `*readme*`, `*help*`), and broken shortcuts pointing to non-existent executable targets.
   - Deduplicates identical applications present in both Common and User folders (preferring the user-specific copy or canonical name).
3. **Shortcut Parsing (`IShellLinkW`)**:
   - Initializes COM (`CoInitializeEx`).
   - Instantiates `CLSID_ShellLink` and queries `IPersistFile` to load `.lnk` files.
   - Extracts target path (`GetPath`), arguments (`GetArguments`), working directory (`GetWorkingDirectory`), and icon location (`GetIconLocation`).
4. **Packaged Apps (UWP/MSIX)**:
   - Includes standard essential Windows packaged applications (e.g., Settings, Calculator, Terminal) using their registered AUMIDs.
   - Activates packaged apps via `IApplicationActivationManager::ActivateApplication`.

### 2.3 Safe Execution (`ShellExecuteExW`)
- Desktop applications and `.lnk` files are launched using `ShellExecuteExW` with `SEE_MASK_DEFAULT`.
- `ShellExecuteExW` preserves Windows elevation semantics (UAC prompt when required), passes arguments, sets the working directory, and invokes default verbs reliably without shell command hacks.
- Packaged apps are launched through `IApplicationActivationManager::ActivateApplication` (`CLSCTX_INPROC_SERVER`).
- Zero runtime PowerShell commands or external process trampolines are used.

### 2.4 Persistence
- User pins and settings are stored at `%LOCALAPPDATA%\StartAllForward\settings.json`.
- The configuration structure serializes pinned application identifiers, launch counts, and recent searches.
- If the file is missing or corrupted, safe defaults are initialized without crashing.

---

## 3. Visual Layout & Theme

- **Dimensions**: Scaled dynamically with DPI (default 420px wide × 560px high at 96 DPI).
- **Search Header**:
  - Live search input box with text cursor and placeholder text ("Type to search applications...").
  - Instant filtering on every keystroke.
- **View Toggle**:
  - Compact tabs for "Pinned" and "All Apps".
- **Application List**:
  - 36px list items containing a 24px icon, application title, and secondary path/category text.
  - Hover background highlight and keyboard focus indicator.
- **Palette**: Windows 11 dark theme:
  - Background: `RGB(28, 28, 30)` / `COLORREF(0x001E1C1C)`
  - Card/Input: `RGB(40, 40, 44)` / `COLORREF(0x002C2828)`
  - Hover: `RGB(52, 53, 58)` / `COLORREF(0x003A3534)`
  - Selected Accent: `RGB(6, 119, 217)` / `COLORREF(0x00D97706)`
  - Text: `RGB(240, 240, 240)` with `RGB(150, 150, 155)` secondary text.

---

## 4. Acceptance Criteria

1. **Discovery**: Accurately indexes installed desktop applications from User and Common Programs folders.
2. **Filtering & Deduplication**: Excludes uninstallers and broken shortcuts; deduplicates identical app entries.
3. **Instant Search**: Typing immediately filters the application list by name and executable path.
4. **Keyboard Navigation**:
   - `Down` / `Up` keys navigate through search/app items.
   - `Enter` launches the currently highlighted application.
   - `Escape` closes the Start Menu.
5. **Mouse Interaction**:
   - Clicking an item launches the application and dismisses the menu.
   - Clicking the Start button toggles the menu open/closed.
   - Clicking outside the menu closes it automatically.
6. **Taskbar Positioning**:
   - Docks above bottom taskbars (`AppBarEdge::Bottom`) and below top taskbars (`AppBarEdge::Top`).
   - Clamped within the primary monitor's work area.
7. **Execution**: Applications launch reliably via `ShellExecuteExW` or `IApplicationActivationManager`.
8. **Persistence**: Pinned apps persist across application restarts in `%LOCALAPPDATA%\StartAllForward\settings.json`.
9. **Zero Regressions**: Milestone 1 window tracking and Milestone 2 AppBar taskbar continue to function without errors.

---

## 5. Verification & Testing Strategy

- **Unit Tests**:
  - Application search index ranking and filtering (exact match, prefix match, substring match).
  - Broken shortcut and uninstaller rejection logic.
  - Settings serialization, deserialization, and corrupted JSON recovery.
- **Automated Verification**:
  - `cargo fmt --all -- --check`
  - `cargo check --workspace`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo build --workspace`
- **Runtime Verification**:
  - Indexing execution time and count of discovered desktop applications.
  - Positioning check for both `--edge top` and `--edge bottom`.
  - Application launch verification.

---

## 6. Implementation Verification Results

| Verification Check | Target | Result | Evidence & Metrics |
|---|---|---|---|
| **Unit Test Suite** | Workspace (`cargo test`) | **PASSED** | **21 passed, 0 failed** across `startallforward-core` (search ranking, pins, settings serialization) and `startallforward-windows` (known folders, shortcut uninstaller rejection, settings save/load and corrupted JSON recovery, window filtering). |
| **Clippy Linting** | `cargo clippy --workspace --all-targets -- -D warnings` | **PASSED** | 0 warnings across all crates. |
| **Code Formatting** | `cargo fmt --all -- --check` | **PASSED** | 0 diffs across all source files. |
| **Application Indexing** | Real Windows host (`--list-apps`) | **PASSED** | Discovered and indexed **155 applications** from User Programs (`%APPDATA%`), Common Programs (`%ProgramData%`), and verified Windows UWP packaged apps. |
| **Top-Edge Docking + Start Menu** | Host display (`--edge top --duration 5`) | **PASSED** | Successfully initialized AppBar, created `StartMenuWindow`, handled Start button click, and cleanly unregistered AppBar and hooks. |
| **Bottom-Edge Docking + Start Menu** | Host display (`--edge bottom --duration 5`) | **PASSED** | Successfully docked above native Windows 11 taskbar, handled Start button click, and executed clean shutdown. |
| **Settings Persistence** | `%LOCALAPPDATA%\StartAllForward\settings.json` | **PASSED** | Verified directory creation and default settings writing at runtime. |

---

## 7. Known Limitations & Next Steps

1. **Packaged Apps Dynamic Discovery**: Currently indexes essential built-in Windows packaged applications (Settings, Terminal, Calculator, Notepad) via verified AUMIDs. Dynamic enumeration of all installed MSIX packages via `PackageManager` COM interfaces can be added in subsequent milestones.
2. **Context Menu on App Items**: Right-clicking an app item in the menu to "Pin to Start", "Run as administrator", or "Open file location".
3. **Advanced Visual Effects**: Adding DWM Acrylic or Mica backdrop blur via `DwmSetWindowAttribute(DWMWA_SYSTEMBACKDROP_TYPE)` for semi-transparent glass rendering.

