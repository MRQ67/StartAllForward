use std::fmt;

/// Unique platform-agnostic identifier for a window.
/// On Windows, this corresponds to an `HWND` represented as an `isize`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WindowId(pub isize);

impl WindowId {
    pub const NULL: Self = Self(0);

    pub fn is_null(self) -> bool {
        self.0 == 0
    }

    pub fn as_isize(self) -> isize {
        self.0
    }
}

impl fmt::Display for WindowId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:08X}", self.0)
    }
}

impl From<isize> for WindowId {
    fn from(val: isize) -> Self {
        Self(val)
    }
}

/// Snapshot of a tracked top-level window.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WindowState {
    pub id: WindowId,
    pub title: String,
    pub process_id: u32,
    pub process_name: String,
    pub class_name: String,
    pub is_visible: bool,
    pub is_minimized: bool,
    pub is_foreground: bool,
    pub is_cloaked: bool,
    pub is_taskbar_eligible: bool,
    pub first_seen_epoch_ms: u64,
    pub last_seen_epoch_ms: u64,
}

impl WindowState {
    pub fn new(id: WindowId, now_ms: u64) -> Self {
        Self {
            id,
            title: String::new(),
            process_id: 0,
            process_name: String::new(),
            class_name: String::new(),
            is_visible: false,
            is_minimized: false,
            is_foreground: false,
            is_cloaked: false,
            is_taskbar_eligible: false,
            first_seen_epoch_ms: now_ms,
            last_seen_epoch_ms: now_ms,
        }
    }
}

/// Events observed on the system or produced during reconciliation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WindowEvent {
    /// A window was discovered or updated with a complete snapshot.
    Snapshot(WindowState),
    /// A window creation event was observed.
    Created {
        id: WindowId,
        initial_state: Option<WindowState>,
    },
    /// A window destruction event was observed.
    Destroyed(WindowId),
    /// The window title changed.
    TitleChanged { id: WindowId, title: String },
    /// Visibility state changed.
    VisibilityChanged { id: WindowId, is_visible: bool },
    /// Minimized state changed.
    MinimizedChanged { id: WindowId, is_minimized: bool },
    /// Foreground focus switched to this window.
    ForegroundChanged(WindowId),
    /// DWM cloaked state changed (e.g. moved between virtual desktops or suspended).
    CloakChanged { id: WindowId, is_cloaked: bool },
    /// Batch reconciliation snapshot of all active top-level windows.
    Reconciled(Vec<WindowState>),
}

/// Statistics and health metrics for the window tracker.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TrackerStats {
    pub total_tracked: usize,
    pub taskbar_eligible: usize,
    pub foreground_id: Option<WindowId>,
    pub foreground_title: String,
    pub events_processed: u64,
    pub reconciliation_count: u64,
}
