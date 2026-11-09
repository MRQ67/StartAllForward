use std::fs::{create_dir_all, File};
use std::io::{Read, Write};
use std::path::PathBuf;
use tracing::{error, info, warn};

use startallforward_core::UserSettings;

/// Manages loading and persisting user configuration under %LOCALAPPDATA%\StartAllForward\settings.json.
pub struct SettingsManager {
    config_path: PathBuf,
}

impl SettingsManager {
    pub fn new() -> Self {
        let base_dir = dirs_local_appdata().unwrap_or_else(|| PathBuf::from("."));
        let app_dir = base_dir.join("StartAllForward");
        let config_path = app_dir.join("settings.json");

        Self { config_path }
    }

    pub fn with_path(path: PathBuf) -> Self {
        Self { config_path: path }
    }

    pub fn config_path(&self) -> &PathBuf {
        &self.config_path
    }

    /// Load user settings from disk or return default settings if missing/corrupted.
    pub fn load(&self) -> UserSettings {
        if !self.config_path.exists() {
            info!(
                "Settings file does not exist at {:?}; creating default settings",
                self.config_path
            );
            let default_settings = UserSettings::default();
            let _ = self.save(&default_settings);
            return default_settings;
        }

        match File::open(&self.config_path) {
            Ok(mut file) => {
                let mut contents = String::new();
                if let Err(e) = file.read_to_string(&mut contents) {
                    warn!(
                        "Failed to read settings file {:?}: {}; using defaults",
                        self.config_path, e
                    );
                    return UserSettings::default();
                }

                match UserSettings::from_json(&contents) {
                    Ok(settings) => settings,
                    Err(e) => {
                        error!(
                            "Failed to parse settings JSON at {:?}: {}; recovering with defaults",
                            self.config_path, e
                        );
                        UserSettings::default()
                    }
                }
            }
            Err(e) => {
                warn!(
                    "Could not open settings file {:?}: {}; using defaults",
                    self.config_path, e
                );
                UserSettings::default()
            }
        }
    }

    /// Save user settings to disk.
    pub fn save(&self, settings: &UserSettings) -> Result<(), String> {
        if let Some(parent) = self.config_path.parent() {
            if !parent.exists() {
                create_dir_all(parent).map_err(|e| {
                    format!("Failed to create settings directory {:?}: {}", parent, e)
                })?;
            }
        }

        let json = settings
            .to_json()
            .map_err(|e| format!("Failed to serialize settings: {}", e))?;
        let mut file = File::create(&self.config_path).map_err(|e| {
            format!(
                "Failed to create settings file {:?}: {}",
                self.config_path, e
            )
        })?;

        file.write_all(json.as_bytes())
            .map_err(|e| format!("Failed to write settings to {:?}: {}", self.config_path, e))?;

        Ok(())
    }
}

impl Default for SettingsManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper to resolve the Windows LocalAppData directory.
fn dirs_local_appdata() -> Option<PathBuf> {
    if let Ok(local_appdata) = std::env::var("LOCALAPPDATA") {
        if !local_appdata.is_empty() {
            return Some(PathBuf::from(local_appdata));
        }
    }

    if let Ok(userprofile) = std::env::var("USERPROFILE") {
        let p = PathBuf::from(userprofile).join("AppData").join("Local");
        if p.exists() {
            return Some(p);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_manager_save_and_load_roundtrip() {
        let temp_dir = std::env::temp_dir().join("startallforward_test_settings");
        let _ = std::fs::remove_dir_all(&temp_dir);
        let path = temp_dir.join("test_settings.json");

        let manager = SettingsManager::with_path(path);
        let mut settings = manager.load();
        settings.pin("my_test_app.exe".to_string());
        manager.save(&settings).expect("Save should succeed");

        let reloaded = manager.load();
        assert!(reloaded.is_pinned("my_test_app.exe"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_settings_manager_corrupted_json_recovery() {
        let temp_dir = std::env::temp_dir().join("startallforward_test_corrupted");
        let _ = std::fs::remove_dir_all(&temp_dir);
        let path = temp_dir.join("corrupted.json");

        let _ = std::fs::create_dir_all(&temp_dir);
        std::fs::write(&path, "{ invalid json content ...").unwrap();

        let manager = SettingsManager::with_path(path);
        let recovered = manager.load();
        assert_eq!(recovered, UserSettings::default());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
