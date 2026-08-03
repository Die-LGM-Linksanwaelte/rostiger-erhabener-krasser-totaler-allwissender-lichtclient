use eframe::egui;
use common::logging::LogLevel::*;
use common::logging::{FileSink, Logger, TerminalSink};
use common::r_log;
use std::process::{Command, Stdio};
use std::fs::File;
use std::net::TcpStream;

fn main() -> eframe::Result<()> {
    Logger::global().add_sink(Box::new(FileSink::new("/tmp/rektal_launcher.log")));
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
        Box::new(|cc| Ok(Box::<LauncherApp>::new(LauncherApp::new(cc.egui_ctx.clone())))),
    )
}

struct LauncherApp {
    workspace_dir: std::path::PathBuf,
    status_msg: String,
}

impl LauncherApp {
    fn new(_ctx: egui::Context) -> Self {
        let current_dir = std::env::current_dir().unwrap_or_default();
        let workspace_dir = if current_dir.file_name().map_or(false, |n| n == "launcher")
            && current_dir.parent().map_or(false, |p| p.join("Cargo.toml").exists()) {
            current_dir.parent().unwrap().to_path_buf()
        } else {
            current_dir
        };

        Self {
            workspace_dir,
            status_msg: String::new(),
        }
    }

    fn spawn_process(&mut self, bin_name: &str) {
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|parent| parent.to_path_buf()))
            .unwrap_or_else(|| std::path::PathBuf::from("."));

        let sibling_exe = exe_dir.join(bin_name);

        let mut cmd = if sibling_exe.exists() {
            Command::new(&sibling_exe)
        } else {
            let mut c = Command::new("cargo");
            c.arg("run").arg("--bin").arg(bin_name);
            c.current_dir(&self.workspace_dir);
            c
        };

        cmd.stdin(Stdio::null());

        let log_out_path = format!("/tmp/rektal_{}_stdout.log", bin_name);
        let log_err_path = format!("/tmp/rektal_{}_stderr.log", bin_name);

        if let Ok(out_file) = File::create(&log_out_path) {
            cmd.stdout(out_file);
        } else {
            cmd.stdout(Stdio::null());
        }

        if let Ok(err_file) = File::create(&log_err_path) {
            cmd.stderr(err_file);
        } else {
            cmd.stderr(Stdio::null());
        }

        match cmd.spawn() {
            Ok(_) => {
                self.status_msg = format!("Successfully spawned {}!", bin_name);
            }
            Err(e) => {
                self.status_msg = format!("Failed to spawn {}: {}", bin_name, e);
            }
        }
    }

    fn is_kernel_running(&self) -> bool {
        TcpStream::connect_timeout(
            &"127.0.0.1:6767".parse().unwrap(),
            std::time::Duration::from_millis(50),
        ).is_ok()
    }
}

impl eframe::App for LauncherApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let kernel_running = self.is_kernel_running();

        egui::CentralPanel::default().show(ctx, |ui| {

            ui.vertical(|ui| {
                // Kernel Status & Spawner
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        let color = if kernel_running {
                            egui::Color32::from_rgb(46, 204, 113) // Green
                        } else {
                            egui::Color32::from_rgb(231, 76, 60) // Red
                        };
                        let (rect, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
                        ui.painter().circle_filled(rect.center(), 5.0, color);

                        ui.label(egui::RichText::new("Kernel:").strong());
                        if kernel_running {
                            ui.label("Running (Port 6767)");
                        } else {
                            ui.label("Stopped");
                        }
                    });

                    ui.add_space(5.0);

                    if ui.button("▶ Start Kernel").clicked() {
                        self.spawn_process("kernel");
                    }
                });

                ui.add_space(10.0);

                ui.group(|ui| {
                    ui.label(egui::RichText::new("GUI:").strong());
                    ui.add_space(5.0);
                    if ui.button("▶ Start GUI").clicked() {
                        self.spawn_process("gui");
                    }
                });

                ui.add_space(10.0);

                if !self.status_msg.is_empty() {
                    ui.colored_label(egui::Color32::LIGHT_BLUE, &self.status_msg);
                }
            });
        });

        ctx.request_repaint_after(std::time::Duration::from_millis(500));
    }
}