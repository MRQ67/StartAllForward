# StartAllForward — Detailed Feature Matrix & Reverse-Engineering Analysis

This matrix documents the features discovered in StartAllBack (version 3.9.28 on Windows 11 Build 26200), analyzes the implementation mechanics observed, and establishes the independent implementation strategy for **StartAllForward**.

All technical findings are classified according to evidence certainty:
- **CONFIRMED**: Proven through binary inspection, API imports/exports, or live runtime execution.
- **OBSERVED**: Verified via active desktop window hierarchy enumeration or registry state.
- **INFERRED**: Deduced with high engineering confidence from known Windows shell architecture.
- **UNKNOWN**: Mechanism not completely isolated or requiring further investigation.

---

## 1. Category A — Taskbar

### Feature A.1: Taskbar Positioning & Alignment
- **Classification**: CONFIRMED
- **Reference Behavior**: Allows docking the taskbar to the Top, Bottom, Left, or Right edge of any monitor; supports centered or left-aligned taskbar buttons.
- **Evidence & Observations**:
  - Live inspection reveals `Shell_TrayWnd` (0x000207DA) at `Rect:(0,738)-(1366,768)` on the primary display.
  - In `StartAllBackX64.dll`, imports include `SHAppBarMessage` (`SHELL32.dll`), `GetMonitorInfoW`, `EnumDisplayMonitors`, and `SetWindowPos`.
- **Relevant Components**: `StartAllBackX64.dll`.
- **Applicable Windows APIs**:
  - `SHAppBarMessage(ABM_NEW, ...)`
  - `SHAppBarMessage(ABM_SETPOS, ...)`
  - `SetWindowPos`, `SystemParametersInfo(SPI_SETWORKAREA, ...)`
- **Estimated Complexity**: Medium.
- **Compatibility Risks**: Multi-DPI scaling across heterogeneous monitors; Windows 11 virtual desktop transitions.
- **Independent Strategy**:
  - StartAllForward registers as an AppBar using `SHAppBarMessage`.
  - Computes work area per monitor via `GetMonitorInfoW` and informs the system of edge reservation.
  - Dynamically recalculates layout upon `WM_DISPLAYCHANGE` or `WM_DPICHANGED`.
- **Acceptance Criteria**: Taskbar docks cleanly to all 4 screen edges on single and multi-monitor setups; desktop work area updates so maximized application windows do not overlap the taskbar.

---

### Feature A.2: Taskbar Size & Icon Sizing
- **Classification**: CONFIRMED
- **Reference Behavior**: User can choose Small, Medium, or Large taskbar heights with corresponding icon scaling (16x16, 24x24, 32x32, 48x48) and configurable icon padding.
- **Evidence & Observations**:
  - Registry entries in `HKCU\Software\StartIsBack`: `TaskbarLargerIcons = 0`, `TaskbarSpacierIcons = -1`.
  - Cache entries in `HKCU\Software\StartIsBack\Cache`: `OrbWidth.96 = 32`, `OrbHeight.96 = 30`, `OrbWidth.120 = 39`, `OrbWidth.144 = 48`.
  - Live window coordinates show `Shell_TrayWnd` height = 30px, with start button height = 30px, width = 32px at 96 DPI.
- **Relevant Components**: `StartAllBackCfg.exe`, `StartAllBackX64.dll`.
- **Applicable Windows APIs**:
  - `GetDpiForWindow`, `GetSystemMetricsForDpi`
  - Per-monitor DPI scaling APIs (`api-ms-win-shcore-scaling-l1-1-1.dll` / `GetDpiForMonitor`)
  - `SHGetImageList` or `SHGetFileInfo` for jumbo/large/small shell icons.
- **Estimated Complexity**: Low to Medium.
- **Compatibility Risks**: Icon blurring if DPI scaling uses non-integer scaling factors (e.g. 125%, 150%, 175%).
- **Independent Strategy**:
  - StartAllForward uses vector (SVG/XAML) base geometries and high-DPI icon retrieval via `SHGetImageList(SHIL_EXTRALARGE)` and `SHIL_LARGE`.
  - Configurable padding is stored in a clean JSON settings profile.
- **Acceptance Criteria**: Crisp rendering across 100%, 125%, 150%, 200% DPI; instant adjustment of taskbar thickness and button spacing without restarting.

---

### Feature A.3: Application Labels & Window Grouping
- **Classification**: OBSERVED & CONFIRMED
- **Reference Behavior**: Full control over window button combination: "Never combine" (always show text labels), "Combine when taskbar is full", or "Always combine".
- **Evidence & Observations**:
  - Live window tree inspection revealed the classic task list control: `0x00040440 [MSTaskListWClass] "Running applications"`.
  - `StartAllBackX64.dll` imports `RegisterShellHookWindow`, `DeregisterShellHookWindow`, `SetWinEventHook`, `UnhookWinEvent`, `GetWindowTextW`, and `InternalGetWindowText`.
- **Relevant Components**: `StartAllBackX64.dll`.
- **Applicable Windows APIs**:
  - `RegisterShellHookWindow` (receives `HSHELL_WINDOWCREATED`, `HSHELL_WINDOWDESTROYED`, `HSHELL_WINDOWACTIVATED`, `HSHELL_REDRAW`, `HSHELL_FLASH`)
  - `SetWinEventHook` (`EVENT_OBJECT_NAMECHANGE`, `EVENT_SYSTEM_FOREGROUND`)
  - `ITaskbarList3` / `ITaskbarList4` COM interfaces.
- **Estimated Complexity**: Medium to High.
- **Compatibility Risks**: UWP / ApplicationFrameHost window title updates and AppUserModelId resolution.
- **Independent Strategy**:
  - StartAllForward hosts an out-of-process Shell Hook window that tracks all top-level windows in a reactive view-model.
  - Custom UI item template supports three states: Icon-only, Icon + Label (fixed width), or Icon + Label (flex width).
  - Groups items by `AppUserModelId` (`GetApplicationUserModelId`) or executable process identity.
- **Acceptance Criteria**: Window open/close/switch events reflect on the taskbar with sub-16ms latency; titles update dynamically; buttons flash on background alert.

---

### Feature A.4: Multi-Monitor Behavior
- **Classification**: OBSERVED & INFERRED
- **Reference Behavior**: Independent taskbars on secondary monitors; configurable to show "All windows on all taskbars", "Only windows open on this display", or "Taskbar where window is open + main taskbar".
- **Evidence & Observations**:
  - `StartAllBackX64.dll` imports `EnumDisplayMonitors`, `GetMonitorInfoW`, `MonitorFromWindow`, `MonitorFromPoint`, and `MonitorFromRect`.
  - Shell window tree contains secondary tray and monitor handlers.
- **Relevant Components**: `StartAllBackX64.dll`.
- **Applicable Windows APIs**:
  - `EnumDisplayMonitors`, `MonitorFromWindow`
  - Per-monitor AppBar registration (`SHAppBarMessage`)
- **Estimated Complexity**: High.
- **Compatibility Risks**: Monitor connect/disconnect, resolution changes, and sleep/wake events triggering invalid window coordinates.
- **Independent Strategy**:
  - Spawn an independent taskbar view instance for each detected monitor.
  - Window tracker filters taskbar items based on `MonitorFromWindow(targetHWnd) == currentMonitorHandle`.
- **Acceptance Criteria**: Secondary taskbars spawn and dismiss cleanly when monitors are connected or unplugged; taskbar buttons appear on the correct screen according to user preference.

---

### Feature A.5: System Tray & Notification Area
- **Classification**: OBSERVED & CONFIRMED
- **Reference Behavior**: Fully functional notification area with hidden icons flyout (chevron), clock/date button, Action Center toggle, input indicator, and show desktop button.
- **Evidence & Observations**:
  - Live window hierarchy confirms:
    - `0x0002049C [TrayNotifyWnd]`
    - `0x00040314 [ToolbarWindow32] "User Promoted Notification Area"`
    - `0x0002040A [TrayClockWClass]`
    - `0x00020482 [SIBTrayButton] "Notification Chevron"`
    - `0x00020406 [SIBTrayButton] "Action Centre"`
    - `0x00020404 [TrayShowDesktopButtonWClass] "Show desktop"`
- **Relevant Components**: `StartAllBackX64.dll`.
- **Applicable Windows APIs**:
  - Standard Shell tray APIs: `Shell_NotifyIconW`, `Shell_NotifyIconGetRect`
  - Tray message pump: `WM_COPYDATA` messages between tray icons and `Shell_TrayWnd`
  - Undocumented tray interfaces for system flyouts (Action Center: `ShellHost.exe`, modern calendar).
- **Estimated Complexity**: Very High.
- **Compatibility Risks**: The system tray is the most tightly coupled subsystem in Explorer. Directly taking over tray icon messages without Explorer can cause missing tray icons or broken third-party icon menus.
- **Independent Strategy**:
  - For the **MVP**: Leave the native Windows 11 system tray intact or docked side-by-side, or host the tray via classic `TrayNotifyWnd` containment.
  - For full standalone mode: Provide a dedicated system tray hosting service communicating via `Shell_NotifyIcon` registration.
- **Acceptance Criteria**: Clock displays accurate time; clicking the clock opens the Windows calendar flyout; tray icons accept left/right click.

---

### Feature A.6: Transparency, Acrylic & Visual Customization
- **Classification**: CONFIRMED
- **Reference Behavior**: Acrylic blur, Mica effect, translucent color tinting, dynamic opacity changes when windows are maximized.
- **Evidence & Observations**:
  - `StartAllBackX64.dll` imports `dwmapi.dll` (`DwmSetWindowAttribute`, `DwmExtendFrameIntoClientArea`) and `USER32.dll` (`SetWindowCompositionAttribute`, `UpdateLayeredWindow`).
  - `.msstyles` files present in `Styles\` (`Plain8.msstyles`, `Windows 7.msstyles`).
- **Relevant Components**: `StartAllBackX64.dll`, `DarkMagicX64.dll`.
- **Applicable Windows APIs**:
  - `DwmSetWindowAttribute` with `DWMWA_SYSTEMBACKDROP_TYPE` (Mica = 2, Acrylic = 3, Tabbed = 4 on Windows 11 22H2+)
  - `SetWindowCompositionAttribute` (undocumented `ACCENT_ENABLE_BLURBEHIND` / `ACCENT_ENABLE_ACRYLICBLURBEHIND`)
- **Estimated Complexity**: Low to Medium.
- **Compatibility Risks**: Changes in undocumented DWM accent policies in Windows 11 24H2 builds.
- **Independent Strategy**:
  - Use documented `DwmSetWindowAttribute(DWMWA_SYSTEMBACKDROP_TYPE)` on Windows 11 as the primary modern backdrop.
  - Fall back to standard layered window composition with alpha channel for custom tinting.
- **Acceptance Criteria**: Seamless acrylic/Mica background; smooth color blending; no visual artifacts or border flickers when moving windows over the taskbar.

---

## 2. Category B — Start Menu

### Feature B.1: Start Menu Layout & Architecture
- **Classification**: OBSERVED & CONFIRMED
- **Reference Behavior**: Classic Windows 7 two-column layout: Left column contains recent/pinned applications and "All Programs" tree; right column contains quick-access links (Documents, Pictures, Control Panel, Settings) and power button.
- **Evidence & Observations**:
  - Live inspection on Desktop reveals the Start Menu window:
    ```
    PID: 10884 | 0x00040440 | Vis: False | Class: DV2ControlHost | Title: "Start menu"
    ```
  - In `StartAllBackX64.dll`, string table contains `DV2ControlHost`, `StartMenu`, `Programs`, `RecentItems`.
- **Relevant Components**: `StartAllBackX64.dll`.
- **Applicable Windows APIs**:
  - `IShellItem`, `IShellFolder`, `SHGetKnownFolderIDList`
  - Win32 standard window controls or DirectUI rendering.
- **Estimated Complexity**: High.
- **Compatibility Risks**: Handling both classic Win32 shortcuts (`.lnk`, `.url`) and modern packaged UWP apps (`shell:AppsFolder`).
- **Independent Strategy**:
  - StartAllForward builds an independent, modular Start Menu window in modern XAML/WPF.
  - Provides interchangeable layout engines: Classic 2-column (Windows 7 style) and Compact launcher (Windows 10 style).
- **Acceptance Criteria**: Clicking the Start orb opens the Start Menu instantly (< 80ms); navigating subfolders in "All Programs" responds smoothly.

---

### Feature B.2: Pinned & Recent Application Listing
- **Classification**: CONFIRMED
- **Reference Behavior**: Drag-and-drop pinning; MRU (Most Recently Used) program calculation; Jump Lists on hover/right-click with recent files and tasks.
- **Evidence & Observations**:
  - Registry key `StartIsBack.AppsFolder` and `StartIsBack.ImmersiveApplication` register `DelegateExecute` COM handlers (`{E5C31EC8-C5E6-4E07-957E-944DB4AAD85E}`).
  - File cache `Cache\appsfolder.dat` records discovered applications.
- **Relevant Components**: `StartAllBackX64.dll`.
- **Applicable Windows APIs**:
  - `IShellItemArray`, `IApplicationActivationManager`
  - Windows Recent items folder: `FOLDERID_Recent` (`%AppData%\Microsoft\Windows\Recent`)
  - Jump List API: `ICustomDestinationList`, `IObjectArray`
- **Estimated Complexity**: High.
- **Compatibility Risks**: UWP apps requiring `PackageFamilyName` and AppUserModelId activation (`ActivateApplication`).
- **Independent Strategy**:
  - Query installed packaged applications via `Windows.Management.Deployment.PackageManager` or `shell:AppsFolder` enumeration.
  - Parse `.lnk` shortcuts using `IShellLinkW` / `IPersistFile`.
  - Maintain a lightweight local database/JSON store for pin order and launch count weights.
- **Acceptance Criteria**: Users can pin/unpin apps; launching any app works correctly for both classic Win32 `.exe` and modern UWP/Store packages.

---

### Feature B.3: Instant Start Search
- **Classification**: CONFIRMED
- **Reference Behavior**: Typing immediately focuses the search box; searches indexed files, applications, and Windows Settings in real time.
- **Evidence & Observations**:
  - Export in `StartAllBackX64.dll`: `CacheSettingsSearchW` (Ordinal 104).
  - Cache file: `Cache\pcsettingssearch.dat` (1,249,576 bytes) stores serialized Windows Settings pages, GUIDs, and search keywords.
- **Relevant Components**: `StartAllBackX64.dll`.
- **Applicable Windows APIs**:
  - Windows Search OLE DB provider (`System.Data.OleDb` connecting to `Provider=Search.CollatorDSO`)
  - Direct Windows Indexer COM query (`ISearchQueryHelper`)
  - Windows Settings deep links (`ms-settings:*`)
- **Estimated Complexity**: Medium to High.
- **Compatibility Risks**: Windows Search Service being disabled or stalled on user machines.
- **Independent Strategy**:
  - Hybrid search pipeline:
    1. In-memory fast match against cached applications and Settings paths (`ms-settings:`).
    2. Asynchronous background query to Windows Indexer for files/documents.
  - Fallback to Everything search integration if Everything is installed (already detected in autoruns on this machine!).
- **Acceptance Criteria**: Keystrokes produce results within 30ms; top result activated on Enter; handles application names and settings keywords.

---

## 3. Category C — File Explorer

### Feature C.1: Command Bar & Ribbon Replacement
- **Classification**: OBSERVED & CONFIRMED
- **Reference Behavior**: Replaces the Windows 11 modern XAML command bar and tab strip with the classic Windows 10 `UIRibbon` or Windows 7 style command bar.
- **Evidence & Observations**:
  - Live inspection of `CabinetWClass` (0x0004062A) demonstrates:
    ```
    [UIRibbonCommandBarDock] "UIRibbonDockTop"
    [UIRibbonCommandBar] "Ribbon"
    [UIRibbonWorkPane] "Ribbon"
    [ReBarWindow32] -> [TravelBand], [UpBand], [Address Band Root], [UniversalSearchBand]
    ```
  - COM registration in `HKCU\Software\Classes\CLSID\{ab0b37ec-56f6-4a0e-a8fd-7a8bf7c2da96}\TreatAs` redirects `CLSID_ExplorerHostCreator` to `{ab0b37ec-56f6-4a0e-a8fd-7a8bf7c2da97}` (`StartAllBackX64.dll`).
  - Assets: `C:\Users\kasab\AppData\Local\StartAllBack\Ribbon\theme-dark\` contains 20 SVG button icons for the restored ribbon.
- **Relevant Components**: `StartAllBackX64.dll`.
- **Applicable Windows APIs**:
  - Windows Ribbon Framework (`IUIFramework`, `IUIRibbon`)
  - Shell command bands (`IExplorerBrowser`, `ICommandBar`)
- **Estimated Complexity**: Very High.
- **Compatibility Risks**: Brittle in-process interception. Microsoft frequently refactors `explorerframe.dll` and `CLSID_ExplorerHostCreator`.
- **Independent Strategy**:
  - **Phase 1 / MVP Scope**: Do not hook Explorer command bars. Keep StartAllForward focused on the Taskbar and Start Menu.
  - **Future Optional Add-on**: If Explorer ribbon restoration is desired, implement as a strictly isolated, user-approved COM shell extension with fail-safe rollback.
- **Acceptance Criteria**: Documented for future expansion; excluded from initial MVP to preserve stability.

---

### Feature C.2: Classic Context Menus
- **Classification**: CONFIRMED
- **Reference Behavior**: Right-clicking anywhere in Explorer or on the Desktop instantly shows the full classic Win32 context menu without clicking "Show more options" (Shift+F10).
- **Evidence & Observations**:
  - Registered CLSID in `HKCU\Software\Classes\CLSID\{865e5e76-ad83-4dca-a109-50dc2113ce9b}\InProcServer32` with an empty default value or pointing to shell DLL.
  - Context menu handler registered in `HKCU\Software\Classes\AllFilesystemObjects\shellex\ContextMenuHandlers\{a2a9545d-a0c2-42b4-9708-a0b2badd77c9}`.
- **Relevant Components**: `StartAllBackX64.dll`.
- **Applicable Windows APIs**:
  - Documented Windows 11 shell behavior: Adding `HKCU\Software\Classes\CLSID\{865e5e76-ad83-4dca-a109-50dc2113ce9b}\InprocServer32` with empty value instructs Windows 11 Explorer to bypass modern XAML context menus natively!
- **Estimated Complexity**: Very Low.
- **Compatibility Risks**: Minimal; this is an established, non-destructive registry toggle supported by Windows 11.
- **Independent Strategy**:
  - Provide a safe, togglable user setting in StartAllForward to enable or disable the classic context menu via this standard user-level CLSID key.
- **Acceptance Criteria**: Enabling the option displays classic context menus immediately; disabling reverts to Windows 11 modern context menus without rebooting.

---

## 4. Category D — Additional Functionality

### Feature D.1: Settings Management & User Interface
- **Classification**: CONFIRMED
- **Reference Behavior**: Dedicated configuration panel with visual previews of taskbar styles, start orb selector, icon size sliders, and feature switches.
- **Evidence & Observations**:
  - `StartAllBackCfg.exe` runs standalone and saves settings to `HKCU\Software\StartIsBack`.
  - Registered in Control Panel via `{FCEA18FF-BC55-4E63-94D7-1B2EFBFE706F}`.
- **Relevant Components**: `StartAllBackCfg.exe`.
- **Applicable Windows APIs**:
  - Standard Win32 or modern UI framework (WPF/WinUI).
- **Estimated Complexity**: Low.
- **Compatibility Risks**: None.
- **Independent Strategy**:
  - Build a clean, modern WPF configuration application (`StartAllForward.Config.exe`) with real-time live preview.
  - Store configuration in structured JSON (`%APPDATA%\StartAllForward\config.json`) with an optional registry mirror for fast inter-process reading.
- **Acceptance Criteria**: Changing any setting in the UI immediately applies changes to the active taskbar without restarting.

---

### Feature D.2: Startup Behavior & Lifecycle Management
- **Classification**: CONFIRMED
- **Reference Behavior**: Launches silently with Windows user login; auto-restarts if Explorer restarts; provides clean uninstallation.
- **Evidence & Observations**:
  - StartAllBack starts via `FolderExtensions` COM invocation inside `explorer.exe`.
  - `StartAllBackCfg.exe /uninstall` restores original registry state and unregisters COM objects.
- **Relevant Components**: `StartAllBackCfg.exe`, `StartAllBackLoaderX64.dll`.
- **Applicable Windows APIs**:
  - `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` or Windows Task Scheduler (logon trigger).
  - Single-instance mutex (`CreateMutexW`).
- **Estimated Complexity**: Low.
- **Compatibility Risks**: Antivirus false-positives if using atypical autorun vectors. Standard `HKCU\Run` or Task Scheduler is universally accepted and safe.
- **Independent Strategy**:
  - Register StartAllForward via a standard user Task Scheduler task or `HKCU\...\Run` entry.
  - Single-instance enforcement via a named Mutex (`Global\StartAllForward_SingleInstance_Mutex`).
  - Monitor Explorer restarts via `RegisterWindowMessage("TaskbarCreated")`.
- **Acceptance Criteria**: App starts silently on login; gracefully shuts down on logout; automatically re-attaches if `explorer.exe` is killed and restarted.
