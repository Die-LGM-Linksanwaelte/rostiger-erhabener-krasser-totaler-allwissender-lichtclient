use eframe::egui;
pub mod terminal;
pub mod universe;
use terminal::TerminalPanel;
use universe::UniversePanel;

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
}
