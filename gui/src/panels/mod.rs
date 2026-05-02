use eframe::egui;

pub mod settings;
pub mod terminal;
pub mod universe;
use universe::UniversePanel; // Importiere UniversePanel

#[derive(Clone, PartialEq)]
pub enum Tab {
    Universe(UniversePanel), // Hält jetzt eine Instanz von UniversePanel
    Terminal,
    Settings,
}

impl Tab {
    // Das ist das, was der Nutzer oben im Reiter liest:
    pub fn title(&self) -> String {
        match self {
            Tab::Universe(panel) => format!("Universum {}", panel.selected_universe),
            Tab::Terminal => "Terminal".to_string(),
            Tab::Settings => "Settings".to_string(),
        }
    }

    // Das ist das, was wir 'egui' als versteckte ID geben:
    pub fn unique_id(&self) -> String {
        match self {
            Tab::Universe(panel) => format!("universe_tab_{}", panel.tab_id),
            Tab::Terminal => "terminal_tab".to_string(),
            Tab::Settings => "settings_tab".to_string(),
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        match self {
            Tab::Universe(panel) => panel.ui(ui), // Ruft die ui-Methode des UniversePanel auf
            Tab::Terminal => terminal::ui(ui),
            Tab::Settings => settings::ui(ui),
        }
    }
}
