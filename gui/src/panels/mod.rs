//! # UI Panels Module
//!
//! This module defines the core panel types and docking tab structures used by the GUI interface.
//! It exposes the [`Tab`] enum encapsulating various panel components like [`UniversePanel`] and [`TerminalPanel`].

use std::fmt;
use std::fmt::{Display, Formatter};
use eframe::egui;
use serde::{Deserialize, Serialize};
use common::logging::LogLevel::Error;
use common::networking::subscription_objects::SubscribeTopic::DMXConfiguration;
use common::r_log;

/// Module implementing the interactive terminal panel tab.
pub mod terminal;
/// Module implementing the visual DMX universe panel tab.
pub mod universe;
/// Module implementing the patch panel tab.
pub mod patch;

use terminal::TerminalPanel;
use universe::UniversePanel;
use patch::PatchPanel;
use crate::controller::{send_ui_event, UiEvent};
use crate::UI_EVENT_SENDER;

/// Enum
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum TabSaveData {
    Terminal,
    Patch,
    Universe { selected_universe: u8 },
}

/// Enum representing all dockable tab types supported in the user interface.
#[derive(Clone)]
pub enum Tab {
    /// Tab displaying a visual DMX universe channel grid.
    Universe(UniversePanel),
    /// Tab displaying an interactive terminal command log.
    Terminal(TerminalPanel),
    /// Tab
    Patch(PatchPanel),
}

impl Display for Tab {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Tab::Universe{ .. } => {
                write!(f, "{}", serde_json::to_string("UniverseTab").unwrap())},
            Tab::Terminal{ .. } => {
                write!(f, "{}", serde_json::to_string("TerminalTab").unwrap())},
            Tab::Patch{ .. } => {
                write!(f, "{}", serde_json::to_string("PatchTab").unwrap())},
        }
    }
}

impl Tab {
    /// Returns the human-readable display title for this tab header.
    pub fn title(&self) -> String {
        match self {
            Tab::Universe(panel) => format!("Universe {}", panel.selected_universe),
            Tab::Terminal(_) => "Terminal".to_string(),
            Tab::Patch(_) => "Patch".to_string(),
        }
    }

    pub fn is_same_type(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }

    /// Returns a unique string identifier for this tab instance used by the dock state.
    pub fn unique_id(&self) -> String {
        match self {
            Tab::Universe(panel) => format!("universe_tab_{}", panel.tab_id),
            Tab::Terminal(panel) => format!("terminal_tab_{}", panel.tab_id),
            Tab::Patch(panel) => format!("patch_tab_{}", panel.tab_id),
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
            Tab::Patch(panel) => panel.ui(ui),
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
            Tab::Patch(_) => {}
        }
    }

    pub fn on_close(&mut self) {
        send_ui_event(UiEvent::CloseTab {
            tab: self.clone(),
        });
    }
}
