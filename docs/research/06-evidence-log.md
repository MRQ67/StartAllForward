# StartAllForward — Reverse-Engineering Evidence Log

This log compiles the verbatim technical evidence, tool invocations, and forensic observations gathered during the research of StartAllBack on Windows 11 Build 26200.

---

## 1. REA MCP Session & Analysis Environment (Phase 0)

- **Tool Call**: `binary_session({ })`
- **REA Server Version**: `rea-agents@6.8.0`
- **Host Platform**: `win32`, architecture `x64`
- **Provider Status**:
  - `ghidra`: Status `unavailable`, reason: `GHIDRA_INSTALL_DIR is not set`.
  - `hopper`: Status `unavailable`, reason: `unsupported_host` (macOS only).
  - `ida`: Status `unavailable`, reason: `not_configured`.
  - `kuna`: Status `unavailable`, reason: `not_configured`.
  - `rea-dotnet-static`: Status `available` (managed PE/CLI inspector).
  - `rea-pe-imports`: Status `available` (native PE import and delay-load table parser).
  - `rea-pe-resources`: Status `available` (native PE resource tree parser).
  - `rea-native-exports`: Status `available` (native PE/ELF export table parser).
- **Outcome**: Binary disassembly engines requiring external local installations were unconfigured. All PE static analysis, symbol export/import tables, resource extraction, and running desktop inspection operations were successfully completed using native REA tools and system facilities.

---

## 2. Reference Target Inventory & Hashes (Phase 1)

### Installed Registry Entry
- **Key**: `HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Uninstall\StartAllBack`
- **DisplayName**: `StartAllBack`
- **DisplayVersion**: `3.9.28` (Product build: `3.9.28.5423`)
- **Publisher**: `startisback.com`
- **InstallLocation**: `C:\Users\kasab\AppData\Local\StartAllBack\`
- **UninstallString**: `C:\Users\kasab\AppData\Local\StartAllBack\StartAllBackCfg.exe /uninstall`

### File Inventory & Verification

| Binary | Size (Bytes) | SHA-256 Digest | PE Type & Architecture |
| :--- | :--- | :--- | :--- |
| `StartAllBackCfg.exe` | 3,518,680 | `a3a991e6d2e3da056eb4dbfcb5c560f78ae8511e4c44835ea03003f29cc64eca` | `pe32-plus` (x86-64 AMD64) |
| `StartAllBackX64.dll` | 1,051,296 | `184ad7f23538c6b78f082589ab5a5c3184e9873a88932d21860792c0a4609a1b` | `pe32-plus` (x86-64 AMD64) |
| `StartAllBackLoaderX64.dll` | 14,744 | `64a5ca4db0633ddd72758ac7624c06ebe522bc42f033c1cb48037e4b6a4bd245` | `pe32-plus` (x86-64 AMD64) |
| `DarkMagicLoaderX64.exe` | 13,720 | `b09befd5e090a498aff088ef6bd9b3c61e28ca16bc83bc2024a28a16f292e754` | `pe32-plus` (x86-64 AMD64) |
| `DarkMagicX64.dll` | 192,936 | `61895c311e7461213849080bbc9549fedd69a4de18669c38ef17c316e9d398ea` | `pe32-plus` (x86-64 AMD64) |
| `UpdateCheck.exe` | 50,400 | — | `pe32-plus` (x86-64 AMD64) |

---

## 3. PE Export & Import Tables (Phase 2)

### A. `StartAllBackX64.dll`
- **Tool**: `inspect_native_exports` (Evidence ID: `ev_9f12feb303991869f6131acd44e9eedd670f3ec836e61e6561497fe2da0468dc`)
- **Exports**:
  - Ordinal 100: `0x18000227c`
  - Ordinal 101: `0x1800023a4`
  - Ordinal 102: `0x180002188`
  - Ordinal 103: `0x180002084`
  - Ordinal 104: `CacheSettingsSearchW` (`0x1800258f0`)
  - Ordinal 105: `DllCanUnloadNow` (`0x18000505c`)
  - Ordinal 106: `DllGetClassObject` (`0x18008f634`)
  - Ordinal 107: `GlassControls` (`0x1800208e4`)
  - Ordinal 108: `LoadSVG` (`0x18004efc0`)
  - Ordinal 109: `LoadSVGOrb` (`0x18004eeb0`)
  - Ordinal 110: `PickGlyphDlg` (`0x1800293f8`)
  - Ordinal 111: `Startup` (`0x18008f42c`)
  - Ordinal 112: `UninstallW` (`0x180072f98`)
  - Ordinal 113: `Uninstall_AllUsersW` (`0x180072fc0`)
- **Key Imports**:
  - `USER32.dll`: `RegisterShellHookWindow`, `DeregisterShellHookWindow`, `SetWindowsHookExW`, `UnhookWindowsHookEx`, `SetWinEventHook`, `UnhookWinEvent`, `UpdateLayeredWindow`, `SetWindowCompositionAttribute`, `GetWindowBand`, `SetWindowBand`, `IsInDesktopWindowBand`, `InternalGetWindowText`, `CalculatePopupWindowPosition`.
  - `SHELL32.dll`: `SHAppBarMessage`, `Shell_NotifyIconW`, `Shell_NotifyIconGetRect`, `SHGetKnownFolderIDList`, `SHCreateItemFromParsingName`, `SHCreateDefaultContextMenu`.
  - `dwmapi.dll`: `DwmExtendFrameIntoClientArea`, `DwmSetWindowAttribute`, `DwmEnableBlurBehindWindow`, `DwmQueryThumbnailSourceSize`.
  - `UxTheme.dll`: `OpenThemeData`, `DrawThemeBackground`, `BeginBufferedPaint`, `EndBufferedPaint`, `GetThemePartSize`.
  - `ntdll.dll`: `RtlQueryWnfStateData`, `RtlSubscribeWnfStateChangeNotification`, `RtlUnsubscribeWnfNotificationWaitForCompletion`.

### B. `StartAllBackLoaderX64.dll`
- **Tool**: `inspect_native_exports` (Evidence ID: `ev_dc74641d61f054d76f49c58007e9e5cab73ffb7d6877a97670bdfc0c34eaec4b`)
- **Exports**: `DllCanUnloadNow` (Ordinal 1), `DllGetClassObject` (Ordinal 2).
- **Key Imports**: `LoadLibraryW`, `GetProcAddress`, `GetShellWindow`, `CreateProcessW`, `PathCchAppend`, `PathCchRemoveFileSpec`, `RegDeleteKeyW`.

### C. `DarkMagicX64.dll`
- **Tool**: `inspect_native_exports` (Evidence ID: `ev_f3dfe46cd57f93916668f4bc0e2812520f075aad7c266bfd964406cff800c52c`)
- **Exports**: `Bootstrap`, `DllCanUnloadNow`, `DllGetClassObject`, `InitDarkMagic`, `IsCompositedTheme`, `LoadRemote`, `SetCustomScrollbar`, `UnroundPtr`.

---

## 4. Live Runtime Observations (Desktop & Window Tree)

### Active Process Module Injection
Enumeration of running process module lists confirmed active injection into the Windows Shell:
- **`explorer.exe` (PID 10884)**:
  - Loaded `C:\Users\kasab\AppData\Local\StartAllBack\StartAllBackLoaderX64.dll`
  - Loaded `C:\Users\kasab\AppData\Local\StartAllBack\StartAllBackX64.dll`
  - Loaded `C:\Users\kasab\AppData\Local\StartAllBack\DarkMagicX64.dll`
- **`ShellHost.exe` (PID 1100)**:
  - Loaded `C:\Users\kasab\AppData\Local\StartAllBack\StartAllBackX64.dll`

### Live Taskbar Window Hierarchy (`Shell_TrayWnd`)
Enumerated via thread-attached `OpenDesktop("Default")` / `EnumChildWindows`:
```
0x000207DA [Shell_TrayWnd] "" Rect:(0,738)-(1366,768) [W=1366, H=30]
├── 0x00020486 [Start] "Start" Rect:(0,738)-(32,768) [W=32, H=30]
├── 0x00020410 [ReBarWindow32] "" Rect:(32,738)-(1070,768) [W=1038, H=30]
│   └── 0x00030408 [MSTaskSwWClass] "Running applications"
│       └── 0x0004040C [MSTaskListWClass] "Running applications"
└── 0x0002049C [TrayNotifyWnd] "" Rect:(1070,738)-(1366,768) [W=296, H=30]
    ├── 0x00020482 [SIBTrayButton] "Notification Chevron"
    ├── 0x00040314 [ToolbarWindow32] "User Promoted Notification Area"
    ├── 0x000302DA [SIBTrayButton] "Touch keyboard"
    ├── 0x0003030C [SIBTrayButton] "Tray Input Indicator"
    ├── 0x0002040A [TrayClockWClass] "" Rect:(1276,738)-(1335,768) [W=59, H=30]
    ├── 0x00020406 [SIBTrayButton] "Action Centre"
    └── 0x00020404 [TrayShowDesktopButtonWClass] "Show desktop"
```

### Live Start Menu & Shell Windows
- `0x00040440 | Class: DV2ControlHost | Title: "Start menu"` (Windows 7 classic Start menu host window hosted in PID 10884).
- `0x00020488 | Class: Button | Title: "Start"` (Start orb button).

### Live File Explorer Window Hierarchy (`CabinetWClass`)
Enumerated on active window `0x0004062A` ("startallforward"):
```
0x0004062A [CabinetWClass] "startallforward"
├── [UIRibbonCommandBarDock] "UIRibbonDockTop"
│   └── [UIRibbonCommandBar] "Ribbon"
│       └── [UIRibbonWorkPane] "Ribbon"
│           └── [NUIPane] -> [NetUIHWND]
├── [WorkerW] -> [ReBarWindow32]
│   ├── [TravelBand] -> [ToolbarWindow32] "Navigation buttons"
│   ├── [UpBand] -> [ToolbarWindow32] "Up band"
│   ├── [Address Band Root] -> [Breadcrumb Parent] -> [ToolbarWindow32] "Address"
│   └── [UniversalSearchBand] -> [Search Box] -> [SearchEditBoxWrapperClass]
└── [ShellTabWindowClass] "startallforward"
    └── [DUIViewWndClassName] -> [DirectUIHWND] -> [SHELLDLL_DefView] "ShellView"
```

---

## 5. Registry Contracts & COM Classes

1. **In-Process Shell Injection Vector**:
   - `HKCU\Software\Classes\Drive\shellex\FolderExtensions\{117E3954-5034-453A-A18B-7B79493646E6}`
   - `DriveMask = 0xff`
   - InProcServer32 = `C:\Users\kasab\AppData\Local\StartAllBack\StartAllBackLoaderX64.dll`
2. **File Explorer Command Bar Interception**:
   - `HKCU\Software\Classes\CLSID\{ab0b37ec-56f6-4a0e-a8fd-7a8bf7c2da96}\TreatAs = {ab0b37ec-56f6-4a0e-a8fd-7a8bf7c2da97}`
   - Note: Target CLSID `{ab0b37ec-56f6-4a0e-a8fd-7a8bf7c2da96}` is `CLSID_ExplorerHostCreator` in `%SystemRoot%\system32\explorerframe.dll`.
3. **Context Menu Bypass**:
   - `HKCU\Software\Classes\CLSID\{865e5e76-ad83-4dca-a109-50dc2113ce9b}\InProcServer32 = C:\Users\kasab\AppData\Local\StartAllBack\StartAllBackX64.dll`
4. **Packaged App Start Actions**:
   - `HKCU\Software\Classes\StartIsBack.AppsFolder\Shell\*\Command` -> `DelegateExecute = {E5C31EC8-C5E6-4E07-957E-944DB4AAD85E}`
   - `HKCU\Software\Classes\StartIsBack.ImmersiveApplication\Shell\*\Command` -> `DelegateExecute = {E5C31EC8-C5E6-4E07-957E-944DB4AAD85E}`
5. **Configuration Storage**:
   - `HKCU\Software\StartIsBack`
   - Values: `WinBuild = 26200`, `WinLangID = 2057`, `SettingsVersion = 6`, `FrameStyle = 1`, `TaskbarLargerIcons = 0`, `TaskbarSpacierIcons = -1`.
