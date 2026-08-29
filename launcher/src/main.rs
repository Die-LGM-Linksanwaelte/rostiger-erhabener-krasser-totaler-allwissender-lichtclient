//! # R.E.K.T.A.Launcher Application
//!
//! Entry point and GUI application for starting, monitoring, and managing
//! REKTAL Kernel server instances and GUI client processes.
//!
//! ## Overview
//! The launcher provides an [`eframe`]/[`egui`]-based user interface allowing developers
//! and users to:
//! - Spawn the **Kernel** server with or without an interactive terminal console.
//! - Spawn the **GUI** client with or without an interactive terminal console.
//! - Launch processes in a fully detached mode so they outlive the launcher.
//! - Observe status feedback and application logs via [`common::logging`].

mod spawn_process;

use crate::spawn_process::{spawn_gui, spawn_kernel};
use common::logging::LogLevel::*;
use common::logging::{FileSink, Logger, TerminalSink};
use common::r_log;
use eframe::egui;
use sysinfo::{ProcessesToUpdate, System};

/// Application entry point for R.E.K.T.A.Launcher.
///
/// Sets up the global logger sinks (`/tmp/rektal_launcher.log` and terminal stdout),
/// configures the native window viewport options, and initializes the [`eframe`] event loop.
///
/// # Errors
///
/// Returns an [`eframe::Result`] if native window creation or event loop initialization fails.
fn main() -> eframe::Result<()> {
    Logger::global().add_sink(Box::new(FileSink::new("rektal_launcher.log")));
    Logger::global().add_sink(Box::new(TerminalSink { cli_prompt: None }));

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 320.0])
            .with_title("R.E.K.T.A.Launcher")
            .with_resizable(false),
        ..Default::default()
    };

    r_log!(Info, "eframe initialized");

    eframe::run_native(
        "R.E.K.T.A.Launcher",
        options,
        Box::new(|cc| {
            Ok(Box::<LauncherApp>::new(LauncherApp::new(
                cc.egui_ctx.clone(),
            )))
        }),
    )
}

/// Primary state structure for the Launcher GUI application.
struct LauncherApp {
    /// Human-readable status message displayed in the UI for user feedback.
    status_msg: String,
    /// Whether to open a visible console window when spawning the Kernel.
    show_kernel_console: bool,
    /// Whether to open a visible console window when spawning the GUI client.
    show_gui_console: bool,
    /// System monitor instance used for querying running processes.
    system: System,
}

impl LauncherApp {
    /// Creates a new [`LauncherApp`] instance with default state, initializes the [`System`] monitor,
    /// and resolves the workspace directory.
    ///
    /// # Arguments
    ///
    /// * `_ctx` - The egui context used for UI rendering and state configuration.
    fn new(_ctx: egui::Context) -> Self {
        let current_dir = std::env::current_dir().unwrap_or_default();
        let mut search_dir = current_dir.clone();
        let mut workspace_dir = current_dir.clone();

        // Search parent directories for the workspace folder containing 'gui' or 'kernel' Cargo.toml
        for _ in 0..5 {
            if search_dir.join("rektal_client").join("Cargo.toml").exists()
                || search_dir.join("rektal_kernel").join("Cargo.toml").exists()
            {
                workspace_dir = search_dir.join("rektal_client");
                break;
            }
            if search_dir.join("Cargo.toml").exists() && search_dir.join("rektal_client").exists() {
                workspace_dir = search_dir.join("rektal_client");
                break;
            }
            if let Some(parent) = search_dir.parent() {
                search_dir = parent.to_path_buf();
            } else {
                break;
            }
        }

        r_log!(Info, "Resolved workspace directory: {:?}", workspace_dir);

        let sys = System::new();

        Self {
            status_msg: String::new(),
            show_kernel_console: false,
            show_gui_console: false,
            system: sys,
        }
    }

    /// Renders the UI section for configuring and launching the REKTAL Kernel.
    ///
    /// Includes a live status indicator (green/red dot), a checkbox for toggling the
    /// terminal console, and a button to spawn the process if not already running.
    ///
    /// # Arguments
    ///
    /// * `ui` - The egui UI builder used to draw the controls.
    fn draw_kernel_group(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    let color = if self.is_kernel_active() {
                        egui::Color32::from_rgb(46, 204, 113) // Green
                    } else {
                        egui::Color32::from_rgb(231, 76, 60) // Red
                    };
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
                    ui.painter().circle_filled(rect.center(), 5.0, color);
                    ui.label(egui::RichText::new("Kernel:").strong());
                });

                ui.add_space(5.0);

                ui.checkbox(&mut self.show_kernel_console, "show kernel console");

                ui.add_space(5.0);



                if ui.add_enabled(!self.is_kernel_active(), egui::Button::new("▶ Start Kernel")).clicked() {
                    if !self.is_kernel_active() {
                        match spawn_kernel(&[], self.show_kernel_console) {
                            Ok(_) => {
                                self.status_msg = "Kernel successfully spawned!".to_string();
                                r_log!(SuccessEvent, "Kernel successfully spawned!");
                            }
                            Err(e) => {
                                self.status_msg = format!("Failed to spawn Kernel: {}", e);
                                r_log!(Error, "Kernel couldn't be spawned: {}", e);
                            }
                        }
                    } else {
                        self.status_msg = "Kernel already spawned!".to_string();
                    }
                }
            })
        });
    }

    /// Renders the UI section for configuring and launching the REKTAL GUI client.
    ///
    /// Includes a live status indicator (green/red dot), a checkbox for toggling the
    /// terminal console, and a button to spawn the process.
    ///
    /// # Arguments
    ///
    /// * `ui` - The egui UI builder used to draw the controls.
    fn draw_gui_group(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    let color = if self.is_gui_active() {
                        egui::Color32::from_rgb(46, 204, 113) // Green
                    } else {
                        egui::Color32::from_rgb(231, 76, 60) // Red
                    };
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
                    ui.painter().circle_filled(rect.center(), 5.0, color);
                    ui.label(egui::RichText::new("GUI:").strong());
                });

                ui.add_space(5.0);

                ui.checkbox(&mut self.show_gui_console, "show gui console");

                ui.add_space(5.0);

                if ui.add_enabled(!self.is_gui_active(), egui::Button::new("▶ Start GUI")).clicked() {
                    match spawn_gui(&[], self.show_gui_console) {
                        Ok(_) => {
                            self.status_msg = "GUI successfully spawned!".to_string();
                            r_log!(SuccessEvent, "GUI successfully spawned!");
                        }
                        Err(e) => {
                            self.status_msg = format!("Failed to spawn GUI: {}", e);
                            r_log!(Error, "GUI couldn't be spawned: {}", e);
                        }
                    }
                }
            })
        });
    }

    /// Checks whether a REKTAL Kernel process is currently active on the host system.
    ///
    /// Iterates through the active system processes cached in [`System`] to check if any process
    /// name begins with `"kernel"`.
    ///
    /// # Returns
    ///
    /// `true` if a matching Kernel process is active, `false` otherwise.
    fn is_kernel_active(&self) -> bool {
        self.system
            .processes()
            .values()
            .any(|p| p.name().to_string_lossy().starts_with("rektal_kernel"))
    }

    /// Checks whether a REKTAL GUI client process is currently active on the host system.
    ///
    /// Iterates through the active system processes cached in [`System`] to check if any process
    /// name begins with `"gui"`.
    ///
    /// # Returns
    ///
    /// `true` if a matching GUI client process is active, `false` otherwise.
    fn is_gui_active(&self) -> bool {
        self.system
            .processes()
            .values()
            .any(|p| p.name().to_string_lossy().starts_with("rektal_client"))
    }
}

impl eframe::App for LauncherApp {
    /// Primary update function called once per UI frame by `eframe`.
    ///
    /// Renders the central panel with side-by-side controls and live status dots for Kernel and GUI,
    /// presents active status messages, refreshes the OS process list, and schedules automatic repaints every 500ms.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                self.draw_kernel_group(ui);

                ui.add_space(10.0);

                self.draw_gui_group(ui);
            });
            if !self.status_msg.is_empty() {
                ui.colored_label(egui::Color32::LIGHT_BLUE, &self.status_msg);
            }
        });
        self.system.refresh_processes(ProcessesToUpdate::All, true);

        ctx.request_repaint_after(std::time::Duration::from_millis(500));
    }
}
