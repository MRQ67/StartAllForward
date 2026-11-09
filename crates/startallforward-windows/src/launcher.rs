use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use tracing::{error, info};
use windows::core::PCWSTR;
use windows::Win32::Foundation::HWND;
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER,
    COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE,
};
use windows::Win32::UI::Shell::{
    ApplicationActivationManager, IApplicationActivationManager, ShellExecuteExW, AO_NONE,
    SEE_MASK_DEFAULT, SHELLEXECUTEINFOW,
};
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

use startallforward_core::AppItem;

/// Safely launch an application (desktop shortcut or packaged UWP/MSIX app).
pub fn launch_app(item: &AppItem) -> Result<(), String> {
    if item.is_packaged {
        if let Some(aumid) = &item.aumid {
            return launch_packaged_app(aumid);
        }
    }

    launch_desktop_app(
        &item.target_path,
        item.arguments.as_deref(),
        item.working_dir.as_deref(),
    )
}

/// Launch a desktop application using documented ShellExecuteExW API.
pub fn launch_desktop_app(
    path: &str,
    arguments: Option<&str>,
    working_dir: Option<&str>,
) -> Result<(), String> {
    info!("Launching desktop application: {:?}", path);

    let path_wide: Vec<u16> = Path::new(path)
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let args_wide: Option<Vec<u16>> = arguments.map(|args| {
        Path::new(args)
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    });

    let dir_wide: Option<Vec<u16>> = working_dir.map(|dir| {
        Path::new(dir)
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    });

    unsafe {
        let mut sei = SHELLEXECUTEINFOW {
            cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
            fMask: SEE_MASK_DEFAULT,
            hwnd: HWND::default(),
            lpVerb: PCWSTR::null(),
            lpFile: PCWSTR(path_wide.as_ptr()),
            lpParameters: args_wide
                .as_ref()
                .map(|v| PCWSTR(v.as_ptr()))
                .unwrap_or(PCWSTR::null()),
            lpDirectory: dir_wide
                .as_ref()
                .map(|v| PCWSTR(v.as_ptr()))
                .unwrap_or(PCWSTR::null()),
            nShow: SW_SHOWNORMAL.0,
            ..Default::default()
        };

        if ShellExecuteExW(&mut sei).is_ok() {
            info!("Successfully launched {:?}", path);
            Ok(())
        } else {
            let err = format!("ShellExecuteExW failed for application {:?}", path);
            error!("{}", err);
            Err(err)
        }
    }
}

/// Launch a packaged application using IApplicationActivationManager.
pub fn launch_packaged_app(aumid: &str) -> Result<(), String> {
    info!("Activating packaged application AUMID: {:?}", aumid);

    let aumid_wide: Vec<u16> = aumid.encode_utf16().chain(std::iter::once(0)).collect();

    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE);

        let res = match CoCreateInstance::<_, IApplicationActivationManager>(
            &ApplicationActivationManager,
            None,
            CLSCTX_INPROC_SERVER,
        ) {
            Ok(manager) => {
                match manager.ActivateApplication(
                    PCWSTR(aumid_wide.as_ptr()),
                    PCWSTR::null(),
                    AO_NONE,
                ) {
                    Ok(pid) => {
                        info!("Packaged app {:?} activated with PID {}", aumid, pid);
                        Ok(())
                    }
                    Err(e) => {
                        let err = format!("ActivateApplication failed for {}: {}", aumid, e);
                        error!("{}", err);
                        Err(err)
                    }
                }
            }
            Err(e) => {
                let err = format!("Failed to create ApplicationActivationManager: {}", e);
                error!("{}", err);
                Err(err)
            }
        };

        CoUninitialize();
        res
    }
}
