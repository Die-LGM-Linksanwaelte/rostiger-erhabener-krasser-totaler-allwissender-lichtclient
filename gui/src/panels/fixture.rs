use eframe::egui;
use common::fixture::{FIXTURE_LIST, SimplePropertyType};

pub fn ui(ui: &mut egui::Ui, fixture_name: &str) {
    ui.heading(format!("🔦 {}", fixture_name));
    ui.add_space(5.0);
    ui.separator();
    ui.add_space(10.0);

    // Zugriff auf die globale Fixture-Liste
    match FIXTURE_LIST.write() {
        Ok(mut list) => {
            if let Some(fixture) = list.fixtures.get_mut(fixture_name) {
                
                // Wir zeigen alle Properties dynamisch an
                egui::ScrollArea::vertical().show(ui, |ui| {
                    
                    // 1. Dimmer zuerst (falls vorhanden)
                    if let Some(channel) = fixture.properties.get_mut(&SimplePropertyType::Dimmer) {
                        ui.group(|ui| {
                            ui.label("💡 Haupt-Dimmer");
                            let mut val = (channel.value >> 8) as u8; // Wir zeigen 0-255 für den User
                            if ui.add(egui::Slider::new(&mut val, 0..=255)).changed() {
                                channel.value = (val as u16) << 8;
                            }
                        });
                        ui.add_space(10.0);
                    }

                    // 2. Alle anderen Properties
                    ui.label("Eigenschaften:");
                    egui::Grid::new("prop_grid").spacing([10.0, 10.0]).show(ui, |ui| {
                        for (prop_type, channel) in fixture.properties.iter_mut() {
                            if *prop_type == SimplePropertyType::Dimmer { continue; }
                            
                            ui.label(format!("{:?}", prop_type));
                            
                            // 16-Bit Slider
                            let mut val = channel.value;
                            if ui.add(egui::Slider::new(&mut val, 0..=65535)).changed() {
                                channel.value = val;
                            }
                            ui.end_row();
                        }
                    });
                });

            } else {
                ui.colored_label(egui::Color32::RED, "Fehler: Fixture nicht in FIXTURE_LIST gefunden.");
            }
        }
        Err(_) => {
            ui.colored_label(egui::Color32::KHAKI, "Warte auf Zugriff (Lock)...");
        }
    }
}
