use eframe::egui;

pub mod fixture;
pub mod settings;
pub mod terminal;
pub mod universe;

#[derive(Clone, PartialEq)]
pub enum Tab {
    Universe {
        tab_id: u32,              // Die feste ID des Fensters (z.B. 0, 1, 2...)
        selected_universe: u32    // Das Universum, das im Dropdown gewählt wurde
    },
    Fixture,
    Terminal,
    Settings,
}
impl Tab {
    // Das ist das, was der Nutzer oben im Reiter liest:
    pub fn title(&self) -> String {
        match self {
            Tab::Terminal => "Terminal".to_string(),
            Tab::Universe { selected_universe, .. } => format!("Universum {}", selected_universe),
            Tab::Settings => "Settings".to_string(),
            _ => {String::from("penis")},
        }
    }

    // Das ist das, was wir 'egui' als versteckte ID geben:
    pub fn unique_id(&self) -> String {
        match self {
            // Nutze hier zwingend die feste tab_id!
            Tab::Universe { tab_id, .. } => format!("universe_tab_{}", tab_id),
            Tab::Fixture => "fixture_tab".to_string(),
            Tab::Terminal => "terminal_tab".to_string(),
            Tab::Settings => "settings_tab".to_string(),
        }
    }
}