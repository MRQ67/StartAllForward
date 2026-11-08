use crossbeam_channel::Sender;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::OnceLock;
use std::thread::{self, JoinHandle};
use tracing::{info, warn};
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::Accessibility::{SetWinEventHook, UnhookWinEvent, HWINEVENTHOOK};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, PostThreadMessageW, TranslateMessage, MSG, WM_QUIT,
};

use crate::api::{capture_window_state, get_window_text};
use startallforward_core::{WindowEvent, WindowId};

const OBJID_WINDOW: i32 = 0x00000000;
const CHILDID_SELF: i32 = 0x00000000;

// WinEvent flags
const WINEVENT_OUTOFCONTEXT: u32 = 0x0000;
const WINEVENT_SKIPOWNPROCESS: u32 = 0x0002;

// WinEvent ranges
const EVENT_SYSTEM_FOREGROUND: u32 = 0x0003;
const EVENT_SYSTEM_MINIMIZESTART: u32 = 0x0016;
const EVENT_SYSTEM_MINIMIZEEND: u32 = 0x0017;
const EVENT_OBJECT_CREATE: u32 = 0x8000;
const EVENT_OBJECT_DESTROY: u32 = 0x8001;
const EVENT_OBJECT_SHOW: u32 = 0x8002;
const EVENT_OBJECT_HIDE: u32 = 0x8003;
const EVENT_OBJECT_NAMECHANGE: u32 = 0x800C;
const EVENT_OBJECT_CLOAKED: u32 = 0x8017;
const EVENT_OBJECT_UNCLOAKED: u32 = 0x8018;

static EVENT_SENDER: OnceLock<Sender<WindowEvent>> = OnceLock::new();

/// Controller handle for managing the background WinEvent hook thread.
pub struct WinEventHookHandle {
    thread_id: u32,
    join_handle: Option<JoinHandle<()>>,
}

impl WinEventHookHandle {
    /// Start the WinEvent hook thread and begin receiving window events.
    pub fn start(sender: Sender<WindowEvent>) -> Result<Self, String> {
        let _ = EVENT_SENDER.set(sender);

        static SPAWNED_TID: AtomicU32 = AtomicU32::new(0);

        let (ready_tx, ready_rx) = crossbeam_channel::bounded::<Result<u32, String>>(1);

        let join_handle = thread::Builder::new()
            .name("saf-winevent-pump".to_string())
            .spawn(move || {
                crate::desktop::try_attach_default_desktop();
                let tid = unsafe { GetCurrentThreadId() };
                SPAWNED_TID.store(tid, Ordering::SeqCst);

                // Register hooks
                // 1. System events (foreground, minimize/restore)
                let hook_sys = unsafe {
                    SetWinEventHook(
                        EVENT_SYSTEM_FOREGROUND,
                        EVENT_SYSTEM_MINIMIZEEND,
                        None,
                        Some(win_event_proc),
                        0,
                        0,
                        WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
                    )
                };

                // 2. Object events (create, destroy, show, hide, namechange, cloaked)
                let hook_obj = unsafe {
                    SetWinEventHook(
                        EVENT_OBJECT_CREATE,
                        EVENT_OBJECT_UNCLOAKED,
                        None,
                        Some(win_event_proc),
                        0,
                        0,
                        WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
                    )
                };

                if hook_sys.is_invalid() || hook_obj.is_invalid() {
                    let err = "Failed to register SetWinEventHook".to_string();
                    let _ = ready_tx.send(Err(err));
                    return;
                }

                info!("WinEvent hooks installed successfully on thread {}", tid);
                let _ = ready_tx.send(Ok(tid));

                // Dedicated Win32 Message Pump
                unsafe {
                    let mut msg = MSG::default();
                    loop {
                        let ret = GetMessageW(&mut msg, None, 0, 0);
                        if ret.0 <= 0 {
                            if ret.0 == -1 {
                                warn!("GetMessageW encountered an error");
                            }
                            break;
                        }
                        let _ = TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }

                    // Clean teardown upon WM_QUIT or exit
                    info!("WinEvent message loop finished; unhooking...");
                    if !hook_sys.is_invalid() {
                        let _ = UnhookWinEvent(hook_sys);
                    }
                    if !hook_obj.is_invalid() {
                        let _ = UnhookWinEvent(hook_obj);
                    }
                }
                info!("WinEvent hooks cleanly unregistered");
            })
            .map_err(|e| format!("Failed to spawn hook thread: {}", e))?;

        let thread_id = ready_rx
            .recv()
            .map_err(|e| format!("Failed to receive hook thread confirmation: {}", e))??;

        Ok(Self {
            thread_id,
            join_handle: Some(join_handle),
        })
    }

    /// Post WM_QUIT to terminate the message loop and wait for hook unregistration.
    pub fn stop(&mut self) {
        if let Some(handle) = self.join_handle.take() {
            unsafe {
                let _ = PostThreadMessageW(self.thread_id, WM_QUIT, WPARAM(0), LPARAM(0));
            }
            let _ = handle.join();
        }
    }
}

impl Drop for WinEventHookHandle {
    fn drop(&mut self) {
        self.stop();
    }
}

unsafe extern "system" fn win_event_proc(
    _h_hook: HWINEVENTHOOK,
    event: u32,
    hwnd: HWND,
    id_object: i32,
    id_child: i32,
    _id_event_thread: u32,
    _dwms_event_time: u32,
) {
    // Only process window-level events (ignore menus, carets, scrollbars, etc.)
    if id_object != OBJID_WINDOW || id_child != CHILDID_SELF {
        return;
    }

    if hwnd.is_invalid() {
        return;
    }

    let Some(sender) = EVENT_SENDER.get() else {
        return;
    };

    let id = WindowId::from(hwnd.0 as isize);
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    match event {
        EVENT_SYSTEM_FOREGROUND => {
            let _ = sender.send(WindowEvent::ForegroundChanged(id));
        }
        EVENT_SYSTEM_MINIMIZESTART => {
            let _ = sender.send(WindowEvent::MinimizedChanged {
                id,
                is_minimized: true,
            });
        }
        EVENT_SYSTEM_MINIMIZEEND => {
            let _ = sender.send(WindowEvent::MinimizedChanged {
                id,
                is_minimized: false,
            });
        }
        EVENT_OBJECT_CREATE => {
            let snapshot = capture_window_state(hwnd, now_ms);
            let _ = sender.send(WindowEvent::Created {
                id,
                initial_state: snapshot,
            });
        }
        EVENT_OBJECT_DESTROY => {
            let _ = sender.send(WindowEvent::Destroyed(id));
        }
        EVENT_OBJECT_SHOW => {
            if let Some(state) = capture_window_state(hwnd, now_ms) {
                let _ = sender.send(WindowEvent::Snapshot(state));
            } else {
                let _ = sender.send(WindowEvent::VisibilityChanged {
                    id,
                    is_visible: true,
                });
            }
        }
        EVENT_OBJECT_HIDE => {
            let _ = sender.send(WindowEvent::VisibilityChanged {
                id,
                is_visible: false,
            });
        }
        EVENT_OBJECT_NAMECHANGE => {
            let title = get_window_text(hwnd);
            let _ = sender.send(WindowEvent::TitleChanged { id, title });
        }
        EVENT_OBJECT_CLOAKED => {
            let _ = sender.send(WindowEvent::CloakChanged {
                id,
                is_cloaked: true,
            });
        }
        EVENT_OBJECT_UNCLOAKED => {
            let _ = sender.send(WindowEvent::CloakChanged {
                id,
                is_cloaked: false,
            });
        }
        _ => {}
    }
}
