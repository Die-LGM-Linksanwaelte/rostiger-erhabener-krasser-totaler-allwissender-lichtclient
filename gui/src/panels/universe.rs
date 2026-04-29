use crate::egui::WidgetType::Label;
use eframe::egui;
use egui_extras::{Column, TableBuilder};

// 'id' ist das aktuell gewählte Universum (mutierbar)
// 'anzahl' ist die Gesamtzahl der verfügbaren Universen

pub(crate) struct UniversePanel {
    panel_id: u32,
    universe: u32,
    universe_count: u32,
}

impl UniversePanel {
    pub fn ui(ui: &mut egui::Ui, id: &mut u32, anzahl: u32) {
        let panel_id = ui.id().with("universe_content_area");

        ui.push_id(panel_id, |ui| {
            // Hier dein restlicher Code...
            ui.horizontal(|ui| {
                ui.label("Universum:");
                // Nutze hier ui.id().with(...) für das Dropdown
                egui::ComboBox::from_id_source(ui.id().with("dropdown"))
                    .selected_text(format!("Universum {}", id))
                    .show_ui(ui, |ui| {
                        for i in 1..=anzahl {
                            ui.selectable_value(id, i, format!("Nr. {}", i));
                        }
                    });
            });

            ui.add_space(8.0); // Kleiner Abstand

            ui.add_space(8.0);

            let cell_width: f32 = 50.0; // Etwas schmaler für reine Labels
            let spacing: f32 = 4.0;
            let available_width = ui.available_width();

            // Berechnen, wie viele Spalten reinpassen
            let num_columns = (available_width / (cell_width + spacing)).floor() as usize;
            let num_columns = num_columns.max(1);

            // In universe.rs
            let row_height = 35.0 + spacing;
            egui::ScrollArea::vertical().show_rows(ui, row_height, (512 / num_columns) + 1, |ui, row_range| {
                egui::Grid::new("dmx_grid")
                    .num_columns(num_columns)
                    .spacing([spacing, spacing])
                    .striped(true)
                    .show(ui, |ui| {
                        // Wir loopen nur durch die sichtbaren Zeilen!
                        for row in row_range {
                            for col in 0..num_columns {
                                let i = row * num_columns + col + 1;
                                if i <= 512 {
                                    draw_dmx_cell(ui, i, 67, cell_width);
                                }
                            }
                            ui.end_row();
                        }
                    });
            });

            fn draw_dmx_cell(ui: &mut egui::Ui, i: usize, val: u8, width: f32) {
                // Ein Frame gibt der Zelle einen Hintergrund und einen abgerundeten Rand
                egui::Frame::none()
                    .fill(ui.visuals().faint_bg_color) // Ganz dezent dunkler/heller als der Rest
                    .rounding(2.0)
                    .inner_margin(2.0)
                    .stroke(ui.visuals().widgets.noninteractive.bg_stroke) // Eine dünne Linie drumherum
                    .show(ui, |ui| {
                        ui.set_min_width(width);
                        ui.vertical_centered(|ui| {
                            ui.label(egui::RichText::new(i.to_string()).size(10.0).weak());
                            ui.label(egui::RichText::new(val.to_string()).strong().size(14.0).monospace());
                        });
                    });
            }

            let width = ui.available_width();
            ui.label(format!("Verfügbare Breite: {}px", width));

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
        });
    }
}
