//! # UI Panels Module
//!
//! This module defines the core panel types and docking tab structures used by the GUI interface.
//! It exposes the [`Tab`] enum encapsulating various panel components like [`UniversePanel`] and [`TerminalPanel`].

use eframe::egui;
use common::logging::LogLevel::Error;
use common::networking::subscription_objects::SubscribeTopic::DMXConfiguration;
use common::r_log;

/// Module implementing the interactive terminal panel tab.
pub mod terminal;
/// Module implementing the visual DMX universe panel tab.
pub mod universe;

use terminal::TerminalPanel;
use universe::UniversePanel;
use crate::controller::UiEvent;
use crate::UI_EVENT_SENDER;

/// Enum representing all dockable tab types supported in the user interface.
#[derive(Clone)]
pub enum Tab {
    /// Tab displaying a visual DMX universe channel grid.
    Universe(UniversePanel),
    /// Tab displaying an interactive terminal command log.
    Terminal(TerminalPanel),
}

impl Tab {
    /// Returns the human-readable display title for this tab header.
    pub fn title(&self) -> String {
        match self {
            Tab::Universe(panel) => format!("Universe {}", panel.selected_universe),
            Tab::Terminal(_) => "Terminal".to_string(),
        }
    }

    /// Returns a unique string identifier for this tab instance used by the dock state.
    pub fn unique_id(&self) -> String {
        match self {
            Tab::Universe(panel) => format!("universe_tab_{}", panel.tab_id),
            Tab::Terminal(panel) => format!("terminal_tab_{}", panel.tab_id),
        }
    }

    /// Renders the content UI for the active tab variant.
    ///
    /// # Arguments
    /// * `ui` - Mutable reference to the `egui::Ui` context.
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        match self {
            Tab::Universe(panel) => panel.ui(ui),
            Tab::Terminal(panel) => panel.ui(ui),
        }
    }

    /// Callback triggered when the application successfully connects and authenticates.
    ///
    /// Initiates necessary server subscriptions (such as requesting DMX configuration updates).
    pub fn on_connect(&mut self) {
        match self {
            Tab::Universe(_) => {
                if let Some(sender) = UI_EVENT_SENDER.read().unwrap().as_ref() {
                    if let Err(e) = sender.send(UiEvent::SubscribeRequest {topic: DMXConfiguration}) {
                        r_log!(Error, "Failed to send UiEvent: {}", e);
                    }
                }
            }
            Tab::Terminal(_) => {}
        }
    }
}
