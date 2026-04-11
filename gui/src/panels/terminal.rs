use eframe::egui;

pub fn ui(ui: &mut egui::Ui) {
    ui.heading("💻 Terminal-Ausgabe");
    ui.add_space(5.0);

    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.monospace("EDER stinkt");
    });
}
