use crate::launcher_models::AppItem;

/// In-memory indexed collection of applications with fast search and ranking.
#[derive(Debug, Clone, Default)]
pub struct AppSearchIndex {
    items: Vec<AppItem>,
}

impl AppSearchIndex {
    pub fn new(items: Vec<AppItem>) -> Self {
        Self { items }
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn all_items(&self) -> &[AppItem] {
        &self.items
    }

    pub fn items_mut(&mut self) -> &mut Vec<AppItem> {
        &mut self.items
    }

    /// Update items with current pinned state from user settings.
    pub fn update_pins(&mut self, pinned_ids: &[String]) {
        for item in &mut self.items {
            item.is_pinned = pinned_ids.iter().any(|id| {
                id.eq_ignore_ascii_case(&item.id)
                    || id.eq_ignore_ascii_case(&item.name)
                    || item
                        .target_path
                        .to_ascii_lowercase()
                        .ends_with(&id.to_ascii_lowercase())
            });
        }
    }

    /// Retrieve pinned applications.
    pub fn pinned_items(&self) -> Vec<AppItem> {
        let mut pinned: Vec<AppItem> = self.items.iter().filter(|i| i.is_pinned).cloned().collect();
        pinned.sort_by(|a, b| {
            b.launch_count
                .cmp(&a.launch_count)
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });
        pinned
    }

    /// Retrieve all applications sorted alphabetically.
    pub fn all_alphabetical(&self) -> Vec<AppItem> {
        let mut all = self.items.clone();
        all.sort_by_key(|a| a.name.to_lowercase());
        all
    }

    /// Perform scored search query.
    pub fn search(&self, query: &str, limit: usize) -> Vec<AppItem> {
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            // Empty query: return pinned items first, then alphabetical
            let mut result = self.pinned_items();
            if result.len() < limit {
                let existing_ids: Vec<String> = result.iter().map(|i| i.id.clone()).collect();
                for item in self.all_alphabetical() {
                    if !existing_ids.contains(&item.id) {
                        result.push(item);
                        if result.len() >= limit {
                            break;
                        }
                    }
                }
            }
            result.truncate(limit);
            return result;
        }

        let mut scored: Vec<(u32, AppItem)> = Vec::new();

        for item in &self.items {
            let name_lower = item.name.to_lowercase();
            let target_lower = item.target_path.to_lowercase();

            let mut score = 0u32;

            if name_lower == q {
                score += 1000;
            } else if name_lower.starts_with(&q) {
                score += 800;
            } else if name_lower
                .split([' ', '-', '_', '.'])
                .any(|word| word.starts_with(&q))
            {
                score += 600;
            } else if name_lower.contains(&q) {
                score += 400;
            } else if target_lower.contains(&q) {
                score += 200;
            }

            if score > 0 {
                if item.is_pinned {
                    score += 50;
                }
                score += item.launch_count.min(100);
                scored.push((score, item.clone()));
            }
        }

        scored.sort_by(|a, b| {
            b.0.cmp(&a.0)
                .then_with(|| a.1.name.to_lowercase().cmp(&b.1.name.to_lowercase()))
        });

        scored
            .into_iter()
            .take(limit)
            .map(|(_, item)| item)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::launcher_models::UserSettings;

    fn sample_apps() -> Vec<AppItem> {
        vec![
            AppItem::new_desktop(
                "code.exe",
                "Visual Studio Code",
                "C:\\Users\\User\\AppData\\Local\\Programs\\Microsoft VS Code\\Code.exe",
                None,
                None,
                None,
                0,
            ),
            AppItem::new_desktop(
                "calc.exe",
                "Calculator",
                "C:\\Windows\\System32\\calc.exe",
                None,
                None,
                None,
                0,
            ),
            AppItem::new_desktop(
                "cmd.exe",
                "Command Prompt",
                "C:\\Windows\\System32\\cmd.exe",
                None,
                None,
                None,
                0,
            ),
            AppItem::new_desktop(
                "chrome.exe",
                "Google Chrome",
                "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe",
                None,
                None,
                None,
                0,
            ),
            AppItem::new_desktop(
                "notepad.exe",
                "Notepad",
                "C:\\Windows\\System32\\notepad.exe",
                None,
                None,
                None,
                0,
            ),
        ]
    }

    #[test]
    fn test_exact_and_prefix_search_ranking() {
        let index = AppSearchIndex::new(sample_apps());

        // "calc" should rank Calculator first with high score
        let res = index.search("calc", 5);
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].name, "Calculator");

        // "code" should rank Visual Studio Code first via word boundary match
        let res = index.search("code", 5);
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].name, "Visual Studio Code");

        // "c" should match Calculator, Command Prompt, Google Chrome, Visual Studio Code
        let res = index.search("c", 5);
        assert!(res.len() >= 3);
        // Prefix matches should rank ahead of substring matches
        assert!(res[0].name.starts_with('C'));
    }

    #[test]
    fn test_empty_query_returns_pinned_or_all() {
        let mut index = AppSearchIndex::new(sample_apps());
        index.update_pins(&["calc.exe".to_string()]);

        let res = index.search("", 5);
        assert_eq!(res.len(), 5);
        // Pinned item should be first
        assert_eq!(res[0].name, "Calculator");
    }

    #[test]
    fn test_pinned_items_retrieval() {
        let mut index = AppSearchIndex::new(sample_apps());
        index.update_pins(&["notepad.exe".to_string(), "cmd.exe".to_string()]);

        let pinned = index.pinned_items();
        assert_eq!(pinned.len(), 2);
        let names: Vec<String> = pinned.iter().map(|p| p.name.clone()).collect();
        assert!(names.contains(&"Notepad".to_string()));
        assert!(names.contains(&"Command Prompt".to_string()));
    }

    #[test]
    fn test_user_settings_serialization() {
        let mut settings = UserSettings::default();
        settings.pin("custom_app.exe".to_string());
        assert!(settings.is_pinned("custom_app.exe"));

        let json = settings.to_json().expect("Serialization failed");
        let restored = UserSettings::from_json(&json).expect("Deserialization failed");
        assert_eq!(settings, restored);
        assert!(restored.is_pinned("custom_app.exe"));

        // Test unpin
        let mut mutable_restored = restored;
        mutable_restored.unpin("custom_app.exe");
        assert!(!mutable_restored.is_pinned("custom_app.exe"));
    }
}
