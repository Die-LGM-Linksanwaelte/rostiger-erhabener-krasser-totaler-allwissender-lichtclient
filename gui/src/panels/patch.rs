
#[derive(Clone)]
pub struct PatchPanel {
    /// Unique tab identifier hosting this terminal panel instance.
    pub tab_id: u32,
}

impl PatchPanel {
    pub fn new(tab_id: u32) -> Self {
        Self {
            tab_id,
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        let panel_id = ui
            .id()
            .with(format!("terminal_content_area_{}", self.tab_id));

        ui.push_id(panel_id, |ui| {
            self.draw_universe_overview(ui);
        });
    }

    fn draw_universe_overview(&self, _ui: &mut egui::Ui) {

    }
}
