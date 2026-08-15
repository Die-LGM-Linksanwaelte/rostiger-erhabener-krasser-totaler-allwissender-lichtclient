use eframe::egui;
use common::logging::LogLevel::Error;
use common::networking::subscription_objects::SubscribeTopic::DMXConfiguration;
use common::r_log;

pub mod terminal;
pub mod universe;
use terminal::TerminalPanel;
use universe::UniversePanel;
use crate::controller::UiEvent;
use crate::UI_EVENT_SENDER;

///enum that contains all tabs of the application
#[derive(Clone)]
pub enum Tab {
    Universe(UniversePanel),
    Terminal(TerminalPanel),
}

///implements the core functions for the Tam Enum
impl Tab {
    pub fn title(&self) -> String {
        match self {
            Tab::Universe(panel) => format!("Universe {}", panel.selected_universe),
            Tab::Terminal(_) => "Terminal".to_string(),
        }
    }

    pub fn unique_id(&self) -> String {
        match self {
            Tab::Universe(panel) => format!("universe_tab_{}", panel.tab_id),
            Tab::Terminal(_) => "terminal_tab".to_string(),
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        match self {
            Tab::Universe(panel) => panel.ui(ui),
            Tab::Terminal(panel) => panel.ui(ui),
        }
    }

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
