use crossbeam_channel::Receiver;
use eframe::egui;
use startallforward_core::{WindowEvent, WindowTracker};
use std::time::Duration;

/// Diagnostic window tracking application.
pub struct DiagnosticApp {
    tracker: WindowTracker,
    event_rx: Receiver<WindowEvent>,
    filter_search: String,
    only_taskbar_eligible: bool,
    active_tab: Tab,
}

#[derive(PartialEq, Eq)]
enum Tab {
    Windows,
    EventLog,
    Architecture,
}

impl DiagnosticApp {
    pub fn new(event_rx: Receiver<WindowEvent>) -> Self {
        Self {
            tracker: WindowTracker::new(500),
            event_rx,
            filter_search: String::new(),
            only_taskbar_eligible: false,
            active_tab: Tab::Windows,
        }
    }

    /// Process all pending events from the background hook and reconciliation channels.
    fn drain_events(&mut self) {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        while let Ok(event) = self.event_rx.try_recv() {
            self.tracker.apply_event(event, now_ms);
        }
    }
}

impl eframe::App for DiagnosticApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.drain_events();

        // Repaint at ~15-20 FPS when idle to keep event feed snappy without burning battery
        ctx.request_repaint_after(Duration::from_millis(50));

        let stats = self.tracker.stats();

        // --- TOP BAR: Header and Metrics ---
        egui::TopBottomPanel::top("top_header").show(ctx, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.heading("🚀 StartAllForward — Milestone 1: Observational Window Tracker");
            });

            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                badge(
                    ui,
                    "Tracked Windows",
                    &format!("{}", stats.total_tracked),
                    egui::Color32::from_rgb(40, 120, 200),
                );
                badge(
                    ui,
                    "Taskbar Eligible",
                    &format!("{}", stats.taskbar_eligible),
                    egui::Color32::from_rgb(40, 160, 80),
                );
                badge(
                    ui,
                    "Events Processed",
                    &format!("{}", stats.events_processed),
                    egui::Color32::from_rgb(120, 80, 180),
                );
                badge(
                    ui,
                    "Reconciliation Cycles",
                    &format!("{}", stats.reconciliation_count),
                    egui::Color32::from_rgb(180, 120, 40),
                );

                if let Some(fg_id) = stats.foreground_id {
                    let fg_text =
                        format!("{} ('{}')", fg_id, truncate(&stats.foreground_title, 25));
                    badge(
                        ui,
                        "Foreground",
                        &fg_text,
                        egui::Color32::from_rgb(200, 70, 70),
                    );
                } else {
                    badge(ui, "Foreground", "None", egui::Color32::GRAY);
                }
            });

            ui.add_space(6.0);
            ui.separator();

            // Navigation tabs and filters
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.active_tab, Tab::Windows, "🪟 Active Windows");
                ui.selectable_value(&mut self.active_tab, Tab::EventLog, "📜 Live WinEvent Log");
                ui.selectable_value(
                    &mut self.active_tab,
                    Tab::Architecture,
                    "ℹ️ Architecture & Safety",
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.active_tab == Tab::Windows {
                        ui.checkbox(&mut self.only_taskbar_eligible, "Taskbar eligible only");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.filter_search)
                                .hint_text("Search title, PID, or HWND..."),
                        );
                        ui.label("🔍");
                    }
                });
            });
            ui.add_space(4.0);
        });

        // --- BOTTOM BAR: Safety notice ---
        egui::TopBottomPanel::bottom("bottom_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("🔒 Safety Mode: Observational only. Zero injection, no registry/taskbar modifications.").weak().small());
            });
        });

        // --- CENTRAL PANEL ---
        egui::CentralPanel::default().show(ctx, |ui| match self.active_tab {
            Tab::Windows => self.show_windows_table(ui),
            Tab::EventLog => self.show_event_log(ui),
            Tab::Architecture => self.show_architecture_info(ui),
        });
    }
}

impl DiagnosticApp {
    fn show_windows_table(&self, ui: &mut egui::Ui) {
        let all_windows = self.tracker.all_windows();
        let search = self.filter_search.to_lowercase();

        let filtered: Vec<_> = all_windows
            .into_iter()
            .filter(|w| {
                if self.only_taskbar_eligible && (!w.is_taskbar_eligible || w.is_cloaked) {
                    return false;
                }
                if !search.is_empty() {
                    let id_str = format!("{}", w.id).to_lowercase();
                    let pid_str = format!("{}", w.process_id);
                    let title_match = w.title.to_lowercase().contains(&search);
                    let proc_match = w.process_name.to_lowercase().contains(&search);
                    let class_match = w.class_name.to_lowercase().contains(&search);
                    title_match
                        || proc_match
                        || class_match
                        || id_str.contains(&search)
                        || pid_str.contains(&search)
                } else {
                    true
                }
            })
            .collect();

        ui.label(format!(
            "Showing {} of {} tracked windows",
            filtered.len(),
            self.tracker.all_windows().len()
        ));
        ui.add_space(4.0);

        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Grid::new("windows_grid")
                    .striped(true)
                    .min_col_width(70.0)
                    .spacing([12.0, 6.0])
                    .show(ui, |ui| {
                        // Header
                        ui.strong("HWND");
                        ui.strong("Title");
                        ui.strong("Process");
                        ui.strong("PID");
                        ui.strong("Class");
                        ui.strong("Taskbar?");
                        ui.strong("Visible");
                        ui.strong("Min");
                        ui.strong("Cloaked");
                        ui.strong("Focus");
                        ui.end_row();

                        for w in filtered {
                            let is_fg = w.is_foreground;
                            let fg_color = if is_fg {
                                egui::Color32::from_rgb(255, 215, 0)
                            } else {
                                ui.visuals().text_color()
                            };

                            ui.colored_label(fg_color, format!("{}", w.id));

                            let display_title = if w.title.is_empty() {
                                "(empty title)".to_string()
                            } else {
                                truncate(&w.title, 40)
                            };
                            ui.colored_label(fg_color, display_title);

                            ui.label(&w.process_name);
                            ui.label(format!("{}", w.process_id));
                            ui.label(truncate(&w.class_name, 20));

                            if w.is_taskbar_eligible {
                                ui.colored_label(egui::Color32::GREEN, "Yes");
                            } else {
                                ui.colored_label(egui::Color32::GRAY, "No");
                            }

                            ui.label(if w.is_visible { "Yes" } else { "No" });
                            ui.label(if w.is_minimized { "Yes" } else { "-" });
                            ui.label(if w.is_cloaked { "Yes" } else { "-" });

                            if is_fg {
                                ui.colored_label(egui::Color32::YELLOW, "★ Active");
                            } else {
                                ui.label("-");
                            }

                            ui.end_row();
                        }
                    });
            });
    }

    fn show_event_log(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(format!(
                "Recent Event Stream (Buffer size: {})",
                self.tracker.recent_log().len()
            ));
        });
        ui.add_space(4.0);

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for entry in self.tracker.recent_log() {
                    let color = if entry.starts_with("FOREGROUND:") {
                        egui::Color32::from_rgb(255, 215, 0)
                    } else if entry.starts_with("CREATED:") {
                        egui::Color32::from_rgb(100, 220, 100)
                    } else if entry.starts_with("DESTROYED:") {
                        egui::Color32::from_rgb(240, 100, 100)
                    } else if entry.starts_with("RECONCILE") {
                        egui::Color32::from_rgb(100, 180, 255)
                    } else {
                        ui.visuals().text_color()
                    };

                    ui.colored_label(color, entry);
                }
            });
    }

    fn show_architecture_info(&self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.heading("StartAllForward Milestone 1 Architecture");
            ui.add_space(6.0);
            ui.label("StartAllForward is designed as a standalone user-mode daemon in Rust.");
            ui.add_space(4.0);
            ui.label("Key Guarantees in Milestone 1:");
            ui.label("• Observational Only: Discovers and monitors windows without modifying system state.");
            ui.label("• Zero Injection: Does not inject DLLs into explorer.exe or any third-party processes.");
            ui.label("• WinEvent Driven: Dedicated Win32 thread pumping SetWinEventHook events out-of-process.");
            ui.label("• Periodic Reconciliation: EnumWindows reconciliation sweep recovers from dropped events.");
            ui.label("• Safe Cleanup: Full hook unregistration upon exit with zero leftover artifacts.");
            ui.add_space(10.0);
            ui.heading("AppBar Evaluation for Production Taskbar:");
            ui.label("• In future milestones, the taskbar will dock using SHAppBarMessage(ABM_NEW).");
            ui.label("• An AppBar requires custom Win32 subclassing to handle ABN_POSCHANGED and edge docking.");
            ui.label("• While egui/eframe provides a fast and responsive diagnostic interface, production taskbar docking will either subclass the native HWND or use a dedicated direct Win32 window.");
        });
    }
}

fn badge(ui: &mut egui::Ui, title: &str, value: &str, color: egui::Color32) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(format!("{}:", title)).strong());
        ui.colored_label(color, egui::RichText::new(value).strong());
    });
}

fn truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let mut truncated: String = s.chars().take(max_chars.saturating_sub(3)).collect();
        truncated.push_str("...");
        truncated
    }
}
