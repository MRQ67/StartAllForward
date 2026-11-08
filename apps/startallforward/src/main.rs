use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use startallforward_core::WindowTracker;
use startallforward_ui::DiagnosticApp;
use startallforward_windows::{AppBarEdge, CustomTaskbar, WindowTrackingService};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize structured logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting StartAllForward (Milestone 2)...");

    let args: Vec<String> = std::env::args().collect();
    let diagnostic_gui = args.iter().any(|arg| arg == "--diagnostic");
    let cli_mode = args.iter().any(|arg| arg == "--cli" || arg == "--headless");
    let test_duration_secs = args
        .windows(2)
        .find(|w| w[0] == "--duration")
        .and_then(|w| w[1].parse::<u64>().ok());

    let edge = if args.iter().any(|arg| arg == "--bottom" || arg == "-b") {
        AppBarEdge::Bottom
    } else if let Some(edge_arg) = args
        .windows(2)
        .find(|w| w[0] == "--edge")
        .map(|w| w[1].to_lowercase())
    {
        if edge_arg == "bottom" {
            AppBarEdge::Bottom
        } else {
            AppBarEdge::Top
        }
    } else {
        AppBarEdge::Top
    };

    let thickness = args
        .windows(2)
        .find(|w| w[0] == "--thickness")
        .and_then(|w| w[1].parse::<i32>().ok())
        .unwrap_or(44);

    // Start background tracking service (2.5s reconciliation cycle)
    let mut service = match WindowTrackingService::start(Duration::from_millis(2500)) {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to initialize Windows tracking service: {}", e);
            return Err(e.into());
        }
    };

    if diagnostic_gui {
        info!("Launching Milestone 1 diagnostic dashboard...");
        run_gui_diagnostic(service)?;
    } else if cli_mode {
        info!("Launching Milestone 1 CLI diagnostic loop...");
        run_cli_diagnostic(&mut service, test_duration_secs)?;
    } else {
        info!(
            "Launching Milestone 2 Custom Taskbar (Edge: {:?}, Thickness: {}px)...",
            edge, thickness
        );
        run_custom_taskbar(&mut service, edge, thickness, test_duration_secs)?;
    }

    info!("StartAllForward terminated cleanly.");
    Ok(())
}

fn run_custom_taskbar(
    service: &mut WindowTrackingService,
    edge: AppBarEdge,
    thickness: i32,
    duration_secs: Option<u64>,
) -> Result<(), Box<dyn std::error::Error>> {
    let rx = service.event_receiver().clone();
    let taskbar = CustomTaskbar::spawn(edge, thickness, rx)?;

    // If an automated duration was requested, trigger closure after N seconds
    if let Some(dur) = duration_secs {
        let taskbar_clone = taskbar;
        std::thread::spawn(move || {
            info!("Automated duration timer set for {} seconds...", dur);
            std::thread::sleep(Duration::from_secs(dur));
            info!("Duration elapsed. Posting close message to taskbar window...");
            taskbar_clone.request_close();
        });
    }

    // Run native Win32 message loop
    taskbar.run_message_loop();

    // Ensure full background service shutdown and AppBar unregistration
    service.shutdown();
    Ok(())
}

fn run_cli_diagnostic(
    service: &mut WindowTrackingService,
    duration_secs: Option<u64>,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Running in CLI diagnostic mode...");
    let running = Arc::new(AtomicBool::new(true));

    let mut tracker = WindowTracker::new(200);
    let rx = service.event_receiver();
    let start_time = Instant::now();
    let mut last_summary = Instant::now();

    while running.load(Ordering::SeqCst) {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        // Process incoming events
        while let Ok(event) = rx.try_recv() {
            tracker.apply_event(event, now_ms);
        }

        // Print periodic summary every 3 seconds
        if last_summary.elapsed() >= Duration::from_secs(3) {
            let stats = tracker.stats();
            println!(
                "\n--- [Status @ +{:.1}s] Tracked: {}, Taskbar Eligible: {}, Events: {}, Reconciles: {} ---",
                start_time.elapsed().as_secs_f32(),
                stats.total_tracked,
                stats.taskbar_eligible,
                stats.events_processed,
                stats.reconciliation_count
            );

            if let Some(fg) = tracker.foreground_window() {
                println!(
                    "Foreground: {} | '{}' ({})",
                    fg.id, fg.title, fg.process_name
                );
            }

            println!("Top Active Taskbar Windows:");
            for w in tracker.taskbar_windows().iter().take(8) {
                let status = if w.is_foreground {
                    "[ACTIVE]"
                } else if w.is_minimized {
                    "[MIN]"
                } else {
                    "[SHOW]"
                };
                println!(
                    "  {} {:<8} {:<18} | PID: {:<6} | '{}'",
                    status, w.id, w.process_name, w.process_id, w.title
                );
            }

            last_summary = Instant::now();
        }

        if let Some(dur) = duration_secs {
            if start_time.elapsed() >= Duration::from_secs(dur) {
                println!(
                    "\nTest duration of {}s elapsed. Finishing verification pass...",
                    dur
                );
                break;
            }
        }

        std::thread::sleep(Duration::from_millis(50));
    }

    service.shutdown();
    Ok(())
}

fn run_gui_diagnostic(service: WindowTrackingService) -> Result<(), Box<dyn std::error::Error>> {
    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 720.0])
            .with_min_inner_size([700.0, 450.0])
            .with_title("StartAllForward — Milestone 1 Diagnostic"),
        ..Default::default()
    };

    let rx = service.event_receiver().clone();

    eframe::run_native(
        "StartAllForward Diagnostic Window Tracker",
        native_options,
        Box::new(|_cc| Ok(Box::new(DiagnosticApp::new(rx)))),
    )
    .map_err(|e| format!("eframe execution failed: {}", e))?;

    Ok(())
}
