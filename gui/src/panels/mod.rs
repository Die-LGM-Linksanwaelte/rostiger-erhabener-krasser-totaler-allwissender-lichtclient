use eframe::egui;

pub mod terminal;
pub mod universe;
use terminal::TerminalPanel;
use universe::UniversePanel; // Importiere TerminalPanel

#[derive(Clone, PartialEq)]
pub enum Tab {
    Universe(UniversePanel),
    Terminal(TerminalPanel),
}

impl Tab {
    // Das ist das, was der Nutzer oben im Reiter liest:
    pub fn title(&self) -> String {
        match self {
            Tab::Universe(panel) => format!("Universe {}", panel.selected_universe),
            Tab::Terminal(_) => "Terminal".to_string(),
        }
    }

    // Das ist das, was wir 'egui' als versteckte ID geben:
    pub fn unique_id(&self) -> String {
        match self {
            Tab::Universe(panel) => format!("universe_tab_{}", panel.tab_id),
            // Wir geben dem Terminal-Tab eine feste ID, da es normalerweise nur einen gibt.
            // Wenn du mehrere Terminals willst, bräuchten wir hier auch eine ID.
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
