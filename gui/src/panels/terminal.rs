use eframe::egui;

pub fn ui(ui: &mut egui::Ui) {
    ui.heading("💻 Terminal-Ausgabe");
    ui.add_space(5.0);
    
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.monospace("> Art-Net Stream gestartet...");
        ui.monospace("> Universes: 0, 1");
        ui.monospace("> Warte auf DMX-Updates...");
    });
}
