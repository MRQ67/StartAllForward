# StartAllBack — Reference Application Inventory & Analysis

## 1. Installation Overview

Read-only inspection of the Windows system registry and local filesystem identified the reference application:

| Attribute | Observed Value |
| :--- | :--- |
| **Application Name** | StartAllBack |
| **Installed Version** | `3.9.28` (Product build: `3.9.28.5423`) |
| **Publisher** | startisback.com / startallback.com |
| **Install Scope** | Current User (`HKCU`) |
| **Install Directory** | `C:\Users\kasab\AppData\Local\StartAllBack\` |
| **Uninstall Command** | `C:\Users\kasab\AppData\Local\StartAllBack\StartAllBackCfg.exe /uninstall` |
| **Registry Uninstall Key** | `HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\StartAllBack` |
| **Configuration Key** | `HKCU\Software\StartIsBack` |
| **Target OS Environment** | Windows 11 Build 26200 (Version 24H2), 64-bit AMD64 |

---

## 2. Component Inventory

The installation consists of native x86-64 (AMD64) binaries, supporting 32-bit compatibility helpers, and skinning assets. None of the binaries are managed .NET assemblies (verified via REA PE header analysis: `format: pe32-plus`, `bits: 64`, `machine: 34404 / amd64`).

```
C:\Users\kasab\AppData\Local\StartAllBack\
├── StartAllBackCfg.exe         (3,518,680 bytes)  - Configuration UI & setup engine
├── StartAllBackLoaderX64.dll   (14,744 bytes)     - In-process shell hook loader (COM server)
├── StartAllBackX64.dll         (1,051,296 bytes)  - Core shell engine & UI implementation
├── DarkMagicLoaderX64.exe      (13,720 bytes)     - 64-bit process injection bootstrap
├── DarkMagicLoaderX86.exe      (13,720 bytes)     - 32-bit process injection bootstrap
├── DarkMagicX64.dll            (192,936 bytes)    - 64-bit Win32 dark theme / control hook
├── DarkMagicX86.dll            (160,424 bytes)    - 32-bit Win32 dark theme / control hook
├── UpdateCheck.exe             (50,400 bytes)     - Background update notifier (COM LocalServer)
├── Cache\                                         - Cached search catalogs & ribbon icons
│   ├── appsfolder.dat                             - Cached UWP / shell app definitions
│   ├── pcsettingssearch.dat                       - Cached modern Settings search index
│   └── ribbon-theme-dark-*                        - Cached rasterized ribbon glyphs
├── Orbs\                                          - Start button graphics
│   ├── clover.svg, w8logo.svg                     - Vector SVG start button definitions
│   ├── e1evenorb-pr.png                           - Raster start orb state sheet
│   └── Windows 7.orb                              - Multi-frame classic start orb bitmap
├── Ribbon\                                        - File Explorer command icons
│   ├── theme-dark\ (*.svg)                        - 20 SVG vector icons for dark theme
│   └── theme-light\ (*.svg)                       - 20 SVG vector icons for light theme
└── Styles\                                        - Visual style skin engines
    ├── Plain8.msstyles                            - Flat Windows 8/10 visual style
    └── Windows 7.msstyles                         - Aero-inspired visual style
```

---

## 3. Detailed Component Architecture & Responsibilities

### A. `StartAllBackLoaderX64.dll` (The Injection Vector)
- **Role**: In-process shell extension loader.
- **Export Table**:
  - `DllGetClassObject` (Ordinal 2, RVA `0x180001050`)
  - `DllCanUnloadNow` (Ordinal 1, RVA `0x180001260`)
- **Imports**: `LoadLibraryW`, `GetProcAddress`, `GetShellWindow`, `CreateProcessW`, `PathCchAppend`, `RegDeleteKeyW`.
- **Registration Mechanism**: Registered as an in-process COM server under:
  ```
  HKCU\Software\Classes\CLSID\{117E3954-5034-453A-A18B-7B79493646E6}\InProcServer32
  ```
  Referenced by Windows Shell via:
  ```
  HKCU\Software\Classes\Drive\shellex\FolderExtensions\{117E3954-5034-453A-A18B-7B79493646E6}
  ```
- **How it Works**: When Windows Explorer initializes or enumerates shell namespace objects (specifically drive folders), Explorer loads all registered `FolderExtensions` COM objects. Once loaded inside `explorer.exe`, `StartAllBackLoaderX64.dll` checks if it is hosted within the main shell process, locates `StartAllBackX64.dll`, and invokes its `Startup` entry point.

### B. `StartAllBackX64.dll` (The Core Engine)
- **Role**: Comprehensive shell hook, taskbar replacement, start menu renderer, and Explorer restyler.
- **Observed Loaded Processes**:
  - `explorer.exe` (PID 10884)
  - `ShellHost.exe` (PID 1100)
- **Export Table**:
  - `Startup` (Ordinal 111, RVA `0x18008f42c`): Shell initialization routine.
  - `DllGetClassObject`, `DllCanUnloadNow` (Ordinals 106, 105): COM server exports.
  - `CacheSettingsSearchW` (Ordinal 104): Settings indexing provider.
  - `GlassControls` (Ordinal 107): Custom GDI/DirectComposition glass styling.
  - `LoadSVG`, `LoadSVGOrb` (Ordinals 108, 109): Vector graphics rasterizer for taskbar & start button.
  - `PickGlyphDlg` (Ordinal 110): Internal glyph selection dialog.
  - `UninstallW`, `Uninstall_AllUsersW` (Ordinals 112, 113): Clean de-registration hooks.
- **Key Imported Windows Subsystems**:
  - **Taskbar & Window Management**: `DeregisterShellHookWindow`, `SetWindowsHookExW`, `UnhookWindowsHookEx`, `SetWinEventHook`, `UnhookWinEvent`, `UpdateLayeredWindow`, `SetWindowCompositionAttribute`, `GetWindowBand`, `SetWindowBand`, `IsInDesktopWindowBand`.
  - **Composition & Theming**: `dwmapi.dll` (`DwmExtendFrameIntoClientArea`, `DwmSetWindowAttribute`, `DwmEnableBlurBehindWindow`), `UxTheme.dll` (`OpenThemeData`, `DrawThemeBackground`, `BeginBufferedPaint`, `GetThemePartSize`).
  - **WNF State Notifications**: `ntdll.dll` (`RtlQueryWnfStateData`, `RtlSubscribeWnfStateChangeNotification`) to monitor system theme changes, tablet posture, and shell state transitions.
  - **Shell & Namespace**: `SHELL32.dll` (`SHAppBarMessage`, `SHGetKnownFolderIDList`, `SHCreateItemFromParsingName`, `SHCreateDefaultContextMenu`, `Shell_NotifyIconGetRect`).

### C. `DarkMagicX64.dll` & `DarkMagicLoaderX64.exe` (The Control Theme Hook)
- **Role**: Win32 common controls skinning engine for dark mode and window rounding.
- **Export Table**:
  - `InitDarkMagic` (Ordinal 4)
  - `SetCustomScrollbar` (Ordinal 7)
  - `UnroundPtr` (Ordinal 8)
  - `IsCompositedTheme` (Ordinal 5)
  - `Bootstrap` (Ordinal 1)
  - `LoadRemote` (Ordinal 6)
- **Responsibility**: Microsoft's classic Win32 controls (scrollbars, standard menus, common dialogs) do not natively support Windows 11 dark mode. DarkMagic hooks window procedures and control rendering to supply dark scrollbars, styled menus, and square/rounded window geometry across desktop applications.

### D. `StartAllBackCfg.exe` (Configuration UI)
- **Role**: Standalone settings utility, configuration wizard, and installer/uninstaller.
- **Architecture**: Native x86-64 executable built with Delphi/VCL and Win32 controls.
- **Resources**:
  - Embedded SVG templates (`DARK10`, `DARK11`, `DARK7`, `LIGHT10`, `LIGHT11`, `LIGHT7`, `TRAY_*`, `INSTALL_*`).
  - 38 localized translation bundles in string/binary tables.
- **Persistence**: Writes settings directly to `HKCU\Software\StartIsBack`.

### E. `UpdateCheck.exe` (Update Agent)
- **Role**: Checks for software updates in the background.
- **Activation**: Registered as COM `LocalServer32` under `{da3306b1-2554-420b-8d0e-6bd29bb4d8ed}` and triggered via Windows scheduled maintenance.

---

## 4. COM Interception & System Registration Matrix

StartAllBack registers several COM redirections under `HKCU\Software\Classes\CLSID`:

| Registered CLSID | System Equivalent / Target | Purpose |
| :--- | :--- | :--- |
| `{117E3954-5034-453A-A18B-7B79493646E6}` | `Drive\shellex\FolderExtensions` | Loads `StartAllBackLoaderX64.dll` into `explorer.exe` upon drive enumeration. |
| `{ab0b37ec-56f6-4a0e-a8fd-7a8bf7c2da97}` | `TreatAs` for `{ab0b37ec-56f6-4a0e-a8fd-7a8bf7c2da96}` (`CLSID_ExplorerHostCreator`) | **Interception of File Explorer top command bar**: Replaces Windows 11 modern XAML/WinUI command bar with classic `UIRibbon`. |
| `{865e5e76-ad83-4dca-a109-50dc2113ce9b}` | Classic context menu override | Restores Windows 10 full context menu without the "Show more options" nested sub-item. |
| `{a2a9545d-a0c2-42b4-9708-a0b2badd77c9}` | `AllFilesystemObjects\shellex\ContextMenuHandlers` | Supplies StartAllBack custom menu items and context verb handling. |
| `{E5C31EC8-C5E6-4E07-957E-944DB4AAD85E}` | `DelegateExecute` on `StartIsBack.AppsFolder` & `ImmersiveApplication` | Handles context actions (pinning, properties, app deletion) for packaged modern apps in the Start menu. |
| `{FCEA18FF-BC55-4E63-94D7-1B2EFBFE706F}` | Control Panel integration | Registers `StartAllBackCfg.exe` as an item in the classic Windows Control Panel. |
| `{da3306b1-2554-420b-8d0e-6bd29bb4d8ed}` | `LocalServer32` for `UpdateCheck.exe` | Out-of-process update check execution. |

---

## 5. Architectural Takeaway for StartAllForward

The analysis proves that StartAllBack is a **classic in-process shell hooker**. It achieves its capabilities by:
1. Being injected into Explorer via a user-level COM extension (`Drive\shellex\FolderExtensions`).
2. Subclassing and manipulating private Explorer windows.
3. Suppressing Windows 11 XAML islands via COM `TreatAs` redirection.

**Critical Implication**: While this yields seamless visual integration, it is the fundamental reason shell utilities frequently crash or break when Microsoft releases Windows 11 Feature Updates (such as 23H2 or 24H2). For StartAllForward, an independent, out-of-process architecture will provide far superior safety, upgrade immunity, and user trust.
