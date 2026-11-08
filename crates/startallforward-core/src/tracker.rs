use crate::models::{TrackerStats, WindowEvent, WindowId, WindowState};
use std::collections::{HashMap, VecDeque};

/// In-memory state tracker for desktop windows.
///
/// Decoupled from any OS or Win32 calls to allow pure unit testing,
/// deterministic verification, and replay of event sequences.
#[derive(Debug, Clone)]
pub struct WindowTracker {
    windows: HashMap<WindowId, WindowState>,
    foreground_id: Option<WindowId>,
    events_processed: u64,
    reconciliation_count: u64,
    event_log: VecDeque<String>,
    max_log_entries: usize,
}

impl Default for WindowTracker {
    fn default() -> Self {
        Self::new(200)
    }
}

impl WindowTracker {
    pub fn new(max_log_entries: usize) -> Self {
        Self {
            windows: HashMap::new(),
            foreground_id: None,
            events_processed: 0,
            reconciliation_count: 0,
            event_log: VecDeque::with_capacity(max_log_entries),
            max_log_entries,
        }
    }

    /// Process a incoming window event and update tracked state.
    pub fn apply_event(&mut self, event: WindowEvent, timestamp_ms: u64) {
        self.events_processed += 1;

        match event {
            WindowEvent::Snapshot(mut state) => {
                let id = state.id;
                state.last_seen_epoch_ms = timestamp_ms;
                if let Some(existing) = self.windows.get_mut(&id) {
                    state.first_seen_epoch_ms = existing.first_seen_epoch_ms;
                    state.is_foreground = existing.is_foreground;
                    *existing = state;
                } else {
                    state.first_seen_epoch_ms = timestamp_ms;
                    self.windows.insert(id, state);
                }
                self.log(format!("SNAPSHOT: Window {} updated", id));
            }
            WindowEvent::Created { id, initial_state } => {
                if let Some(mut state) = initial_state {
                    state.last_seen_epoch_ms = timestamp_ms;
                    state.first_seen_epoch_ms = timestamp_ms;
                    self.log(format!("CREATED: Window {} ('{}')", id, state.title));
                    self.windows.insert(id, state);
                } else if !self.windows.contains_key(&id) {
                    let state = WindowState::new(id, timestamp_ms);
                    self.log(format!("CREATED: Window {} (stub)", id));
                    self.windows.insert(id, state);
                }
            }
            WindowEvent::Destroyed(id) => {
                if let Some(removed) = self.windows.remove(&id) {
                    self.log(format!("DESTROYED: Window {} ('{}')", id, removed.title));
                } else {
                    self.log(format!("DESTROYED: Window {} (untracked)", id));
                }
                if self.foreground_id == Some(id) {
                    self.foreground_id = None;
                }
            }
            WindowEvent::TitleChanged { id, title } => {
                if let Some(w) = self.windows.get_mut(&id) {
                    let old = std::mem::replace(&mut w.title, title.clone());
                    w.last_seen_epoch_ms = timestamp_ms;
                    self.log(format!(
                        "TITLE: Window {} changed from '{}' to '{}'",
                        id, old, title
                    ));
                }
            }
            WindowEvent::VisibilityChanged { id, is_visible } => {
                if let Some(w) = self.windows.get_mut(&id) {
                    w.is_visible = is_visible;
                    w.last_seen_epoch_ms = timestamp_ms;
                    self.log(format!("VISIBILITY: Window {} visible={}", id, is_visible));
                }
            }
            WindowEvent::MinimizedChanged { id, is_minimized } => {
                if let Some(w) = self.windows.get_mut(&id) {
                    w.is_minimized = is_minimized;
                    w.last_seen_epoch_ms = timestamp_ms;
                    self.log(format!(
                        "MINIMIZED: Window {} minimized={}",
                        id, is_minimized
                    ));
                }
            }
            WindowEvent::ForegroundChanged(id) => {
                // Clear old foreground
                if let Some(old_id) = self.foreground_id.take() {
                    if let Some(old_w) = self.windows.get_mut(&old_id) {
                        old_w.is_foreground = false;
                    }
                }

                if !id.is_null() {
                    self.foreground_id = Some(id);
                    let title = if let Some(w) = self.windows.get_mut(&id) {
                        w.is_foreground = true;
                        w.last_seen_epoch_ms = timestamp_ms;
                        Some(w.title.clone())
                    } else {
                        None
                    };

                    if let Some(title) = title {
                        self.log(format!("FOREGROUND: Window {} ('{}')", id, title));
                    } else {
                        self.log(format!("FOREGROUND: Window {} (not yet fully tracked)", id));
                    }
                } else {
                    self.log("FOREGROUND: Cleared (desktop/null)".to_string());
                }
            }
            WindowEvent::CloakChanged { id, is_cloaked } => {
                if let Some(w) = self.windows.get_mut(&id) {
                    w.is_cloaked = is_cloaked;
                    w.last_seen_epoch_ms = timestamp_ms;
                    self.log(format!("CLOAK: Window {} cloaked={}", id, is_cloaked));
                }
            }
            WindowEvent::Reconciled(snapshot) => {
                self.reconciliation_count += 1;
                let mut current_ids = std::collections::HashSet::with_capacity(snapshot.len());

                for mut state in snapshot {
                    let id = state.id;
                    current_ids.insert(id);
                    state.last_seen_epoch_ms = timestamp_ms;

                    if let Some(existing) = self.windows.get_mut(&id) {
                        state.first_seen_epoch_ms = existing.first_seen_epoch_ms;
                        state.is_foreground = self.foreground_id == Some(id);
                        *existing = state;
                    } else {
                        state.first_seen_epoch_ms = timestamp_ms;
                        state.is_foreground = self.foreground_id == Some(id);
                        self.windows.insert(id, state);
                    }
                }

                // Prune windows that are no longer in the snapshot
                let before_count = self.windows.len();
                self.windows.retain(|id, _| current_ids.contains(id));
                let pruned = before_count.saturating_sub(self.windows.len());

                if let Some(fg_id) = self.foreground_id {
                    if !self.windows.contains_key(&fg_id) {
                        self.foreground_id = None;
                    }
                }

                self.log(format!(
                    "RECONCILE #{}: Active={}, Pruned={}",
                    self.reconciliation_count,
                    self.windows.len(),
                    pruned
                ));
            }
        }
    }

    /// Retrieve a reference to a window by ID.
    pub fn get(&self, id: WindowId) -> Option<&WindowState> {
        self.windows.get(&id)
    }

    /// Retrieve all tracked windows, sorted by ID.
    pub fn all_windows(&self) -> Vec<&WindowState> {
        let mut list: Vec<&WindowState> = self.windows.values().collect();
        list.sort_by_key(|w| w.id);
        list
    }

    /// Retrieve only windows eligible for display on the taskbar.
    pub fn taskbar_windows(&self) -> Vec<&WindowState> {
        let mut list: Vec<&WindowState> = self
            .windows
            .values()
            .filter(|w| w.is_taskbar_eligible && !w.is_cloaked)
            .collect();
        list.sort_by_key(|w| w.first_seen_epoch_ms);
        list
    }

    /// Get current foreground window state.
    pub fn foreground_window(&self) -> Option<&WindowState> {
        self.foreground_id.and_then(|id| self.windows.get(&id))
    }

    /// Generate summary statistics.
    pub fn stats(&self) -> TrackerStats {
        let eligible = self
            .windows
            .values()
            .filter(|w| w.is_taskbar_eligible && !w.is_cloaked)
            .count();
        let fg = self.foreground_window();
        TrackerStats {
            total_tracked: self.windows.len(),
            taskbar_eligible: eligible,
            foreground_id: self.foreground_id,
            foreground_title: fg.map(|w| w.title.clone()).unwrap_or_default(),
            events_processed: self.events_processed,
            reconciliation_count: self.reconciliation_count,
        }
    }

    /// Recent event log entries (newest first).
    pub fn recent_log(&self) -> &VecDeque<String> {
        &self.event_log
    }

    fn log(&mut self, entry: String) {
        if self.event_log.len() >= self.max_log_entries {
            self.event_log.pop_back();
        }
        self.event_log.push_front(entry);
    }
}
