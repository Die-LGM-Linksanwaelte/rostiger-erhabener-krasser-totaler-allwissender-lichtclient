use eframe::egui;

pub mod fixture;
pub mod terminal;
pub mod universe;

#[derive(Clone, PartialEq)]
pub enum Tab {
    Fixture(String),
    Universe(u32),
    Terminal,
}

impl Tab {
    pub fn title(&self) -> String {
        match self {
            Tab::Fixture(name) => format!("🔦 {}", name),
            Tab::Universe(id) => format!("🌌 Universe {}", id),
            Tab::Terminal => "💻 Terminal".to_string(),
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        match self {
            Tab::Fixture(name) => fixture::ui(ui, name),
            Tab::Universe(id) => universe::ui(ui, id, 5),
            Tab::Terminal => terminal::ui(ui),
        }
    }
}
