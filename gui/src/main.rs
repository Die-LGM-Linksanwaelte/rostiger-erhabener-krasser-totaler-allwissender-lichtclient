//! # R.E.K.T.A.L. GUI Main Application
//!
//! Desktop entry point for the R.E.K.T.A.L. lighting control GUI.
//! Handles startup initialization, logger configuration, window icon embedding,
//! `eframe` viewport setup, and starts the event loop with [`RektalGui`].

use common::logging::LogLevel::*;
use common::logging::{FileSink, Logger, TerminalSink};
use common::r_log;
use eframe::egui;
use gui::RektalGui;

/// Application main entry point.
///
/// Initializes logging sinks, embeds the window icon, configures native viewport options,
/// and starts the `eframe` event loop with [`RektalGui`].
fn main() -> eframe::Result<()> {
    let log_path = std::env::temp_dir().join("rektal_gui.log");
    Logger::global().add_sink(Box::new(FileSink::new(
        log_path.to_str().unwrap_or("/tmp/rektal_gui.log"),
    )));
    Logger::global().add_sink(Box::new(TerminalSink { cli_prompt: None }));

    let icon = {
        let image = img_crate::load_from_memory(include_bytes!(
            "../assets/rektal-logo-without-title-32px.png"
        ))
        .expect("Icon konnte nicht geladen werden")
        .into_rgba8();
        let width = image.width();
        let height = image.height();
        egui::viewport::IconData {
            rgba: image.into_raw(),
            width,
            height,
        }
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 768.0])
            .with_title("R.E.K.T.A.L.")
            .with_app_id("rektal")
            .with_icon(std::sync::Arc::new(icon)),
        ..Default::default()
    };
    r_log!(Info, "eframe initialized");

    eframe::run_native(
        "Docking App",
        options,
        Box::new(|cc| Ok(Box::new(RektalGui::new(cc.egui_ctx.clone())))),
    )
}