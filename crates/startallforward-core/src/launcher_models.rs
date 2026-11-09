use serde::{Deserialize, Serialize};

/// Represents an installed desktop application or packaged app in the launcher.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppItem {
    /// Unique canonical identifier (e.g., lowercase file path or AUMID).
    pub id: String,
    /// Human-readable display name.
    pub name: String,
    /// Target path (executable path, .lnk path, or protocol).
    pub target_path: String,
    /// Command line arguments (if defined by shortcut).
    pub arguments: Option<String>,
    /// Working directory (if defined by shortcut).
    pub working_dir: Option<String>,
    /// Path to file containing the icon (exe, dll, or ico).
    pub icon_path: Option<String>,
    /// Index of icon within the icon file.
    pub icon_index: i32,
    /// Whether this is a packaged UWP/MSIX application.
    pub is_packaged: bool,
    /// App User Model ID (for packaged applications).
    pub aumid: Option<String>,
    /// Whether this application is pinned to the Start Menu.
    pub is_pinned: bool,
    /// Usage counter to prioritize frequently used applications.
    pub launch_count: u32,
}

impl AppItem {
    pub fn new_desktop(
        id: impl Into<String>,
        name: impl Into<String>,
        target_path: impl Into<String>,
        arguments: Option<String>,
        working_dir: Option<String>,
        icon_path: Option<String>,
        icon_index: i32,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            target_path: target_path.into(),
            arguments,
            working_dir,
            icon_path,
            icon_index,
            is_packaged: false,
            aumid: None,
            is_pinned: false,
            launch_count: 0,
        }
    }

    pub fn new_packaged(
        id: impl Into<String>,
        name: impl Into<String>,
        aumid: impl Into<String>,
        icon_path: Option<String>,
    ) -> Self {
        let aumid_str = aumid.into();
        Self {
            id: id.into(),
            name: name.into(),
            target_path: aumid_str.clone(),
            arguments: None,
            working_dir: None,
            icon_path,
            icon_index: 0,
            is_packaged: true,
            aumid: Some(aumid_str),
            is_pinned: false,
            launch_count: 0,
        }
    }
}

/// Start Menu view mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum StartMenuView {
    #[default]
    Pinned,
    AllApps,
}

/// User configuration and persistent launcher settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserSettings {
    /// Ordered list of pinned application IDs.
    pub pinned_app_ids: Vec<String>,
    /// Maximum search results to display in search mode.
    pub max_search_results: usize,
    /// Default view mode when opening Start Menu.
    pub default_view: StartMenuView,
    /// Recent search queries.
    pub recent_searches: Vec<String>,
}

impl Default for UserSettings {
    fn default() -> Self {
        Self {
            pinned_app_ids: vec![
                // Common default pins if present
                "notepad.exe".to_string(),
                "calc.exe".to_string(),
                "cmd.exe".to_string(),
                "explorer.exe".to_string(),
            ],
            max_search_results: 15,
            default_view: StartMenuView::Pinned,
            recent_searches: Vec::new(),
        }
    }
}

impl UserSettings {
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn is_pinned(&self, id: &str) -> bool {
        self.pinned_app_ids
            .iter()
            .any(|p| p.eq_ignore_ascii_case(id))
    }

    pub fn pin(&mut self, id: String) {
        if !self.is_pinned(&id) {
            self.pinned_app_ids.push(id);
        }
    }

    pub fn unpin(&mut self, id: &str) {
        self.pinned_app_ids.retain(|p| !p.eq_ignore_ascii_case(id));
    }
}
