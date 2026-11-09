use std::collections::HashSet;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use tracing::{debug, info};
use windows::core::{Interface, PCWSTR};
use windows::Win32::Foundation::MAX_PATH;
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, IPersistFile, CLSCTX_INPROC_SERVER,
    COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE, STGM_READ,
};
use windows::Win32::UI::Shell::{
    FOLDERID_CommonPrograms, FOLDERID_Programs, IShellLinkW, SHGetKnownFolderPath, ShellLink,
    KF_FLAG_DEFAULT,
};

use startallforward_core::AppItem;

/// Resolve a Known Folder ID to a PathBuf.
pub fn get_known_folder_path(folder_id: &windows::core::GUID) -> Option<PathBuf> {
    unsafe {
        if let Ok(path_ptr) = SHGetKnownFolderPath(folder_id, KF_FLAG_DEFAULT, None) {
            let str_val = path_ptr.to_string().ok();
            windows::Win32::System::Com::CoTaskMemFree(Some(path_ptr.0 as *const _));
            str_val.map(PathBuf::from)
        } else {
            None
        }
    }
}

/// Parsed shortcut metadata.
#[derive(Debug, Clone)]
pub struct ShortcutInfo {
    pub target_path: String,
    pub arguments: Option<String>,
    pub working_dir: Option<String>,
    pub icon_path: Option<String>,
    pub icon_index: i32,
}

/// Parse a .lnk shortcut file using COM IShellLinkW and IPersistFile.
pub fn parse_shortcut(lnk_path: &Path) -> Option<ShortcutInfo> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE);

        let shell_link: IShellLinkW = match CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)
        {
            Ok(link) => link,
            Err(e) => {
                debug!("Failed to create IShellLink instance: {}", e);
                CoUninitialize();
                return None;
            }
        };

        let persist_file: IPersistFile = match shell_link.cast() {
            Ok(pf) => pf,
            Err(e) => {
                debug!("Failed to cast IShellLinkW to IPersistFile: {}", e);
                CoUninitialize();
                return None;
            }
        };

        let path_wide: Vec<u16> = lnk_path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        if let Err(e) = persist_file.Load(PCWSTR(path_wide.as_ptr()), STGM_READ) {
            debug!("Failed to load shortcut {:?}: {}", lnk_path, e);
            CoUninitialize();
            return None;
        }

        let mut target_buf = [0u16; MAX_PATH as usize];
        let target_path = if shell_link
            .GetPath(&mut target_buf, std::ptr::null_mut(), 0)
            .is_ok()
        {
            let len = target_buf
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(target_buf.len());
            String::from_utf16_lossy(&target_buf[..len])
        } else {
            String::new()
        };

        let mut args_buf = [0u16; 1024];
        let arguments = if shell_link.GetArguments(&mut args_buf).is_ok() {
            let len = args_buf
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(args_buf.len());
            let s = String::from_utf16_lossy(&args_buf[..len]);
            if s.trim().is_empty() {
                None
            } else {
                Some(s.trim().to_string())
            }
        } else {
            None
        };

        let mut dir_buf = [0u16; MAX_PATH as usize];
        let working_dir = if shell_link.GetWorkingDirectory(&mut dir_buf).is_ok() {
            let len = dir_buf
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(dir_buf.len());
            let s = String::from_utf16_lossy(&dir_buf[..len]);
            if s.trim().is_empty() {
                None
            } else {
                Some(s)
            }
        } else {
            None
        };

        let mut icon_path_buf = [0u16; MAX_PATH as usize];
        let mut icon_index = 0i32;
        let icon_path = if shell_link
            .GetIconLocation(&mut icon_path_buf, &mut icon_index)
            .is_ok()
        {
            let len = icon_path_buf
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(icon_path_buf.len());
            let s = String::from_utf16_lossy(&icon_path_buf[..len]);
            if s.trim().is_empty() {
                None
            } else {
                Some(s)
            }
        } else {
            None
        };

        CoUninitialize();

        Some(ShortcutInfo {
            target_path,
            arguments,
            working_dir,
            icon_path,
            icon_index,
        })
    }
}

/// Check if a shortcut target or filename is an uninstaller or auxiliary file to ignore.
pub fn is_unwanted_entry(name: &str, target_path: &str) -> bool {
    let name_lower = name.to_lowercase();
    let target_lower = target_path.to_lowercase();

    let unwanted_keywords = [
        "uninstall",
        "unins000",
        "remove",
        "documentation",
        "readme",
        "release notes",
        "help",
        "manual",
        "eula",
        "license",
        "website",
    ];

    for kw in &unwanted_keywords {
        if name_lower.contains(kw) || target_lower.contains(kw) {
            return true;
        }
    }

    if target_lower.ends_with(".txt")
        || target_lower.ends_with(".chm")
        || target_lower.ends_with(".html")
        || target_lower.ends_with(".htm")
        || target_lower.ends_with(".pdf")
        || target_lower.ends_with(".rtf")
    {
        return true;
    }

    false
}

/// Recursively scan a folder for .lnk shortcuts.
pub fn scan_shortcuts_recursive(
    dir: &Path,
    out_apps: &mut Vec<AppItem>,
    seen_names: &mut HashSet<String>,
) {
    if !dir.is_dir() {
        return;
    }

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            debug!("Cannot read directory {:?}: {}", dir, e);
            return;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_shortcuts_recursive(&path, out_apps, seen_names);
        } else if let Some(ext) = path.extension() {
            if ext.eq_ignore_ascii_case("lnk") {
                let file_stem = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string();

                if file_stem.is_empty() {
                    continue;
                }

                let normalized_name = file_stem.trim().to_string();
                let name_key = normalized_name.to_lowercase();

                if seen_names.contains(&name_key) {
                    continue; // Skip duplicate
                }

                if let Some(info) = parse_shortcut(&path) {
                    if is_unwanted_entry(&normalized_name, &info.target_path) {
                        continue;
                    }

                    // Check if target executable exists if it's a direct file path
                    let target_to_launch =
                        if !info.target_path.is_empty() && Path::new(&info.target_path).exists() {
                            info.target_path.clone()
                        } else if path.exists() {
                            // Fall back to launching the .lnk file itself
                            path.to_string_lossy().to_string()
                        } else {
                            continue; // Broken shortcut
                        };

                    seen_names.insert(name_key.clone());

                    out_apps.push(AppItem::new_desktop(
                        path.to_string_lossy().to_string(),
                        normalized_name,
                        target_to_launch,
                        info.arguments,
                        info.working_dir,
                        info.icon_path
                            .or_else(|| Some(path.to_string_lossy().to_string())),
                        info.icon_index,
                    ));
                }
            }
        }
    }
}

/// Discover installed desktop applications and common packaged apps.
pub fn discover_installed_apps() -> Vec<AppItem> {
    info!("Starting application discovery pass...");
    let mut apps = Vec::new();
    let mut seen_names = HashSet::new();

    // 1. Scan User Start Menu Programs
    if let Some(user_programs) = get_known_folder_path(&FOLDERID_Programs) {
        info!("Scanning User Programs: {:?}", user_programs);
        scan_shortcuts_recursive(&user_programs, &mut apps, &mut seen_names);
    }

    // 2. Scan Common / All Users Start Menu Programs
    if let Some(common_programs) = get_known_folder_path(&FOLDERID_CommonPrograms) {
        info!("Scanning Common Programs: {:?}", common_programs);
        scan_shortcuts_recursive(&common_programs, &mut apps, &mut seen_names);
    }

    // 3. Add Verified Built-in Windows Applications
    let builtins = vec![
        AppItem::new_packaged(
            "windows.settings",
            "Settings",
            "windows.immersivecontrolpanel_cw5n1h2txyewy!microsoft.windows.immersivecontrolpanel",
            None,
        ),
        AppItem::new_packaged(
            "windows.terminal",
            "Terminal",
            "Microsoft.WindowsTerminal_8wekyb3d8bbwe!App",
            None,
        ),
        AppItem::new_packaged(
            "windows.calculator",
            "Calculator",
            "Microsoft.WindowsCalculator_8wekyb3d8bbwe!App",
            None,
        ),
        AppItem::new_desktop("notepad.exe", "Notepad", "notepad.exe", None, None, None, 0),
        AppItem::new_desktop("cmd.exe", "Command Prompt", "cmd.exe", None, None, None, 0),
        AppItem::new_desktop(
            "explorer.exe",
            "File Explorer",
            "explorer.exe",
            None,
            None,
            None,
            0,
        ),
        AppItem::new_desktop(
            "taskmgr.exe",
            "Task Manager",
            "taskmgr.exe",
            None,
            None,
            None,
            0,
        ),
    ];

    for builtin in builtins {
        let key = builtin.name.to_lowercase();
        if !seen_names.contains(&key) {
            seen_names.insert(key);
            apps.push(builtin);
        }
    }

    apps.sort_by_key(|a| a.name.to_lowercase());
    info!(
        "Application discovery completed: {} applications discovered",
        apps.len()
    );
    apps
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unwanted_entry_filtering() {
        assert!(is_unwanted_entry(
            "Uninstall Slack",
            "C:\\Slack\\unins000.exe"
        ));
        assert!(is_unwanted_entry("Release Notes", "C:\\App\\notes.txt"));
        assert!(is_unwanted_entry("Help Documentation", "C:\\App\\help.chm"));
        assert!(!is_unwanted_entry(
            "Google Chrome",
            "C:\\Chrome\\chrome.exe"
        ));
        assert!(!is_unwanted_entry(
            "Visual Studio Code",
            "C:\\VSCode\\Code.exe"
        ));
    }

    #[test]
    fn test_known_folders_resolution() {
        let user_programs = get_known_folder_path(&FOLDERID_Programs);
        assert!(
            user_programs.is_some(),
            "FOLDERID_Programs should resolve on Windows"
        );
        let path = user_programs.unwrap();
        assert!(path.exists(), "Programs folder should exist on disk");
    }
}
