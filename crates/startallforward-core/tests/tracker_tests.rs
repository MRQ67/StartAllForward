use startallforward_core::{WindowEvent, WindowId, WindowState, WindowTracker};

fn make_window(id_raw: isize, title: &str, eligible: bool) -> WindowState {
    let id = WindowId::from(id_raw);
    let mut w = WindowState::new(id, 1000);
    w.title = title.to_string();
    w.process_id = 1234;
    w.process_name = "test_app.exe".to_string();
    w.class_name = "TestClass".to_string();
    w.is_visible = true;
    w.is_minimized = false;
    w.is_taskbar_eligible = eligible;
    w
}

#[test]
fn test_tracker_creation_and_snapshot() {
    let mut tracker = WindowTracker::new(50);
    let w1 = make_window(0x1001, "Editor Window", true);

    tracker.apply_event(WindowEvent::Snapshot(w1.clone()), 1000);

    let retrieved = tracker.get(WindowId::from(0x1001)).expect("window found");
    assert_eq!(retrieved.title, "Editor Window");
    assert_eq!(retrieved.process_name, "test_app.exe");
    assert!(retrieved.is_taskbar_eligible);
    assert_eq!(tracker.all_windows().len(), 1);
    assert_eq!(tracker.taskbar_windows().len(), 1);
}

#[test]
fn test_window_lifecycle_and_foreground() {
    let mut tracker = WindowTracker::new(50);
    let id1 = WindowId::from(0x1001);
    let id2 = WindowId::from(0x1002);

    let w1 = make_window(0x1001, "Window 1", true);
    let w2 = make_window(0x1002, "Window 2", true);

    tracker.apply_event(
        WindowEvent::Created {
            id: id1,
            initial_state: Some(w1),
        },
        1000,
    );
    tracker.apply_event(
        WindowEvent::Created {
            id: id2,
            initial_state: Some(w2),
        },
        1010,
    );

    assert_eq!(tracker.all_windows().len(), 2);

    // Switch focus to Window 1
    tracker.apply_event(WindowEvent::ForegroundChanged(id1), 1020);
    assert_eq!(tracker.foreground_window().map(|w| w.id), Some(id1));
    assert!(tracker.get(id1).unwrap().is_foreground);
    assert!(!tracker.get(id2).unwrap().is_foreground);

    // Switch focus to Window 2
    tracker.apply_event(WindowEvent::ForegroundChanged(id2), 1030);
    assert_eq!(tracker.foreground_window().map(|w| w.id), Some(id2));
    assert!(!tracker.get(id1).unwrap().is_foreground);
    assert!(tracker.get(id2).unwrap().is_foreground);

    // Destroy active foreground window
    tracker.apply_event(WindowEvent::Destroyed(id2), 1040);
    assert_eq!(tracker.all_windows().len(), 1);
    assert!(tracker.foreground_window().is_none());
}

#[test]
fn test_title_and_visibility_changes() {
    let mut tracker = WindowTracker::new(50);
    let id = WindowId::from(0x2000);
    let w = make_window(0x2000, "Initial Title", true);

    tracker.apply_event(WindowEvent::Snapshot(w), 1000);

    // Change title
    tracker.apply_event(
        WindowEvent::TitleChanged {
            id,
            title: "Updated Title".to_string(),
        },
        1050,
    );
    assert_eq!(tracker.get(id).unwrap().title, "Updated Title");

    // Change visibility
    tracker.apply_event(
        WindowEvent::VisibilityChanged {
            id,
            is_visible: false,
        },
        1060,
    );
    assert!(!tracker.get(id).unwrap().is_visible);

    // Change minimized
    tracker.apply_event(
        WindowEvent::MinimizedChanged {
            id,
            is_minimized: true,
        },
        1070,
    );
    assert!(tracker.get(id).unwrap().is_minimized);
}

#[test]
fn test_reconciliation_cleans_stale_and_updates() {
    let mut tracker = WindowTracker::new(50);
    let id_kept = WindowId::from(0x3001);
    let id_stale = WindowId::from(0x3002);
    let id_new = WindowId::from(0x3003);

    // Pre-populate with 2 windows
    tracker.apply_event(
        WindowEvent::Snapshot(make_window(0x3001, "Kept Window", true)),
        1000,
    );
    tracker.apply_event(
        WindowEvent::Snapshot(make_window(0x3002, "Stale Window", true)),
        1000,
    );
    tracker.apply_event(WindowEvent::ForegroundChanged(id_stale), 1005);

    assert_eq!(tracker.all_windows().len(), 2);
    assert_eq!(tracker.foreground_window().map(|w| w.id), Some(id_stale));

    // Reconcile snapshot only containing kept window (updated) and a brand new window
    let mut updated_kept = make_window(0x3001, "Kept Window (Updated)", true);
    updated_kept.first_seen_epoch_ms = 9999; // Should be preserved from original
    let new_window = make_window(0x3003, "New Window Discovered", true);

    tracker.apply_event(
        WindowEvent::Reconciled(vec![updated_kept, new_window]),
        2000,
    );

    // Verify stale window was purged
    assert!(tracker.get(id_stale).is_none());
    assert_eq!(tracker.all_windows().len(), 2);
    assert!(tracker.get(id_kept).is_some());
    assert_eq!(tracker.get(id_kept).unwrap().title, "Kept Window (Updated)");
    // Original first seen timestamp is preserved
    assert_eq!(tracker.get(id_kept).unwrap().first_seen_epoch_ms, 1000);
    assert!(tracker.get(id_new).is_some());

    // Foreground belonged to stale window, so it was cleared
    assert!(tracker.foreground_window().is_none());
}

#[test]
fn test_cloaking_and_taskbar_eligibility() {
    let mut tracker = WindowTracker::new(50);
    let id1 = WindowId::from(0x4001);
    let _id2 = WindowId::from(0x4002);

    let w1 = make_window(0x4001, "Normal App", true);
    let w2 = make_window(0x4002, "Background Tool Window", false);

    tracker.apply_event(WindowEvent::Snapshot(w1), 1000);
    tracker.apply_event(WindowEvent::Snapshot(w2), 1000);

    assert_eq!(tracker.all_windows().len(), 2);
    assert_eq!(tracker.taskbar_windows().len(), 1);

    // Now cloak the normal app (e.g., virtual desktop switch)
    tracker.apply_event(
        WindowEvent::CloakChanged {
            id: id1,
            is_cloaked: true,
        },
        1050,
    );
    assert!(tracker.get(id1).unwrap().is_cloaked);
    // Cloaked window must not appear on active taskbar
    assert_eq!(tracker.taskbar_windows().len(), 0);

    // Uncloak
    tracker.apply_event(
        WindowEvent::CloakChanged {
            id: id1,
            is_cloaked: false,
        },
        1100,
    );
    assert_eq!(tracker.taskbar_windows().len(), 1);
}

#[test]
fn test_event_log_bounds() {
    let mut tracker = WindowTracker::new(5);
    for i in 0..20 {
        let id = WindowId::from(i);
        tracker.apply_event(
            WindowEvent::Created {
                id,
                initial_state: None,
            },
            1000 + i as u64,
        );
    }

    assert_eq!(tracker.recent_log().len(), 5);
}
