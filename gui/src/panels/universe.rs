use eframe::egui;

// 'id' ist das aktuell gewählte Universum (mutierbar)
// 'anzahl' ist die Gesamtzahl der verfügbaren Universen
pub fn ui(ui: &mut egui::Ui, id: &mut u32, anzahl: u32) {
    ui.horizontal(|ui| {
        ui.label("Universum:");

        egui::ComboBox::from_id_source("universe_dropdown")
            .selected_text(format!("Universum {}", id))
            .show_ui(ui, |ui| {
                // Die Schleife läuft jetzt bis 'anzahl'
                for i in 1..=anzahl {
                    ui.selectable_value(id, i, format!("Nr. {}", i));
                }
            });
    });

    ui.separator();

    // Die Buttons unten passen sich jetzt auch der Anzahl an
    ui.label(format!("Schnellauswahl (1 bis {})", anzahl));
    ui.horizontal_wrapped(|ui| {
        for i in 1..=anzahl {
            let is_selected = *id == i;
            if ui.selectable_label(is_selected, format!("{}", i)).clicked() {
                *id = i;
            }
        }
    });
}
