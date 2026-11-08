use crossbeam_channel::{bounded, Receiver};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;
use tracing::info;

use crate::enumerator::enumerate_desktop_windows;
use crate::hook::WinEventHookHandle;
use startallforward_core::WindowEvent;

/// Main coordinator service for window tracking, hooks, and reconciliation.
pub struct WindowTrackingService {
    event_rx: Receiver<WindowEvent>,
    hook_handle: Option<WinEventHookHandle>,
    reconcile_thread: Option<JoinHandle<()>>,
    running: Arc<AtomicBool>,
}

impl WindowTrackingService {
    /// Start the service with a specified reconciliation interval.
    pub fn start(reconcile_interval: Duration) -> Result<Self, String> {
        let (tx, rx) = bounded::<WindowEvent>(1024);
        let running = Arc::new(AtomicBool::new(true));

        // 1. Initial enumeration
        info!("Performing initial desktop window enumeration...");
        let initial_windows = thread::spawn(|| {
            crate::desktop::try_attach_default_desktop();
            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);
            enumerate_desktop_windows(now_ms)
        })
        .join()
        .unwrap_or_default();

        info!(
            "Initial enumeration found {} windows",
            initial_windows.len()
        );
        let _ = tx.send(WindowEvent::Reconciled(initial_windows));

        // 2. Start WinEvent hook
        info!("Installing system WinEvent hooks...");
        let hook_handle = WinEventHookHandle::start(tx.clone())?;

        // 3. Start reconciliation thread
        let running_clone = running.clone();
        let reconcile_tx = tx.clone();
        let reconcile_thread = thread::Builder::new()
            .name("saf-reconciliation".to_string())
            .spawn(move || {
                crate::desktop::try_attach_default_desktop();
                info!(
                    "Periodic reconciliation thread started (interval: {:?})",
                    reconcile_interval
                );
                while running_clone.load(Ordering::Relaxed) {
                    thread::sleep(reconcile_interval);
                    if !running_clone.load(Ordering::Relaxed) {
                        break;
                    }

                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_millis() as u64)
                        .unwrap_or(0);

                    let snapshot = enumerate_desktop_windows(now);
                    if reconcile_tx
                        .send(WindowEvent::Reconciled(snapshot))
                        .is_err()
                    {
                        break; // Channel closed
                    }
                }
                info!("Reconciliation thread exited");
            })
            .map_err(|e| format!("Failed to spawn reconciliation thread: {}", e))?;

        Ok(Self {
            event_rx: rx,
            hook_handle: Some(hook_handle),
            reconcile_thread: Some(reconcile_thread),
            running,
        })
    }

    /// Access the receiver channel delivering live window events to the UI.
    pub fn event_receiver(&self) -> &Receiver<WindowEvent> {
        &self.event_rx
    }

    /// Stop all hooks, background threads, and release OS resources.
    pub fn shutdown(&mut self) {
        self.running.store(false, Ordering::Relaxed);

        if let Some(mut hook) = self.hook_handle.take() {
            hook.stop();
        }

        if let Some(thread) = self.reconcile_thread.take() {
            let _ = thread.join();
        }

        info!("WindowTrackingService fully stopped");
    }
}

impl Drop for WindowTrackingService {
    fn drop(&mut self) {
        self.shutdown();
    }
}
