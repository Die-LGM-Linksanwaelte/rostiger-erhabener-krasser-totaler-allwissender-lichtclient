use crate::network::udp_client::MAX_CHANNEL;
use eframe::egui;

#[derive(Clone)]
pub struct UniversePanel {
    pub tab_id: u32,
    pub selected_universe: u8,
    pub dmx_data: [u8; MAX_CHANNEL],
    pub settings_open: bool,
    pub min_pure_cell_width: f32,
}

impl UniversePanel {
    //TODO: refractoring, einzelne methoden usw.
    pub fn new(tab_id: u32) -> Self {
        
        Self {
            tab_id,
            selected_universe: 1,
            dmx_data: [0; MAX_CHANNEL],
            settings_open: false,
            min_pure_cell_width: 45.0, // gewünschte Mindestbreite
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        // --- 1. EINSTELLUNGS-DIALOG ---
        if self.settings_open {
            //TODO save/ discard button hinzufügen, siehe auch: terminal panel
            let mut is_open = self.settings_open;

            egui::Window::new(format!("Settings - Panel {}", self.tab_id)) //TODO: Einstellungsmöglichkeiten hinzufügen
                .open(&mut is_open) // Hier wird is_open geliehen
                .collapsible(false)
                .resizable(false)
                .pivot(egui::Align2::CENTER_CENTER)
                .show(ui.ctx(), |ui| {
                    ui.label("Hier kommen deine Einstellungen hin...");
                    false
                });

            self.settings_open = is_open;
        }

        let panel_id = ui
            .id()
            .with(format!("universe_content_area_{}", self.tab_id));

        ui.push_id(panel_id, |ui| {
            // --- HEADER ---
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Universe").strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("⚙").on_hover_text("Settings").clicked() {
                        self.settings_open = true;
                    }
                });
            });

            // --- SCHNELLAUSWAHL ---
            ui.horizontal_wrapped(|ui| {
                for i in 1..17 {
                    let is_selected = self.selected_universe == i;
                    if ui.selectable_label(is_selected, format!("{}", i)).clicked() {
                        self.selected_universe = i;
                        // Hier würde normalerweise das Daten-Update triggern
                    }
                }
            });

            ui.add_space(8.0);

            // --- DMX GRID BERECHNUNG (FLUID & STRETCHED) ---
            let spacing = 4.0;
            let frame_extra = 6.0; // Platz für Margin und Stroke
            let min_full_cell_width = self.min_pure_cell_width + frame_extra;

            let available_width = (ui.available_width() - 4.0).max(0.0);

            // 1. Wie viele Spalten passen bei der MINDEST-Breite rein?
            // Wir addieren oben ein 'spacing' dazu, weil hinter der letzten Spalte kein Spacing mehr kommt.
            let num_columns =
                ((available_width + spacing) / (min_full_cell_width + spacing)).floor() as usize;
            let num_columns = num_columns.max(1);

            // 2. STRETCH-MATHEMATIK: Berechne die exakte Breite, um den Platz 100% auszufüllen
            // Der gesamte Platz für Abstände zwischen den Spalten:
            let total_spacing = num_columns.saturating_sub(1) as f32 * spacing;
            // Der Platz, der jetzt noch für die Zellen selbst übrig ist, geteilt durch die Anzahl der Zellen:
            let stretched_full_cell_width =
                (available_width - total_spacing) / (num_columns as f32);

            // Die reine Innen-Breite, die wir an draw_dmx_cell übergeben müssen:
            let stretched_pure_cell_width = stretched_full_cell_width - frame_extra;

            // 3. Zeilen und Höhe berechnen (wie vorhin, damit unten kein Loch entsteht)
            let total_rows = (MAX_CHANNEL + num_columns - 1) / num_columns;
            let row_height = 30.0 + spacing; // Die echte Höhe deiner Zelle!

            egui::ScrollArea::vertical()
                .id_source(format!("dmx_scroll_{}", self.tab_id))
                .auto_shrink([false, false])
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                .show_rows(ui, row_height, total_rows, |ui, row_range| {
                    egui::Grid::new(format!("dmx_grid_{}", self.tab_id))
                        .num_columns(num_columns)
                        .spacing([spacing, spacing])
                        // WICHTIG: Wir zwingen das Grid hier exakt auf unsere gestreckte Breite!
                        .min_col_width(stretched_full_cell_width)
                        .max_col_width(stretched_full_cell_width)
                        .show(ui, |ui| {
                            for row in row_range {
                                for col in 0..num_columns {
                                    let i = row * num_columns + col;
                                    if i < MAX_CHANNEL {
                                        // Hier übergeben wir jetzt die dynamisch berechnete "Stretched" Breite
                                        draw_dmx_cell(
                                            ui,
                                            i + 1,
                                            self.dmx_data[i],
                                            stretched_pure_cell_width,
                                        );
                                    } else {
                                        ui.label("");
                                    }
                                }
                                ui.end_row();
                            }
                        });
                });
        });
    }
}

///function that draws a single DMX-Channel-Cell
fn draw_dmx_cell(ui: &mut egui::Ui, channel_num: usize, val: u8, width: f32) {
    egui::Frame::none()
        .fill(ui.visuals().faint_bg_color)
        .rounding(2.0)
        .inner_margin(2.0) // Dies verbraucht Platz INNERHALB der Zelle
        .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
        .show(ui, |ui| {
            // Wir setzen die Breite des Inhalts. Der Frame drumherum macht es breiter.
            ui.set_width(width);
            ui.vertical_centered(|ui| {
                ui.label(
                    egui::RichText::new(channel_num.to_string())
                        .size(9.0)
                        .weak(),
                );
                ui.label(
                    egui::RichText::new(val.to_string())
                        .strong()
                        .size(14.0)
                        .monospace(),
                );
            });
        });
}
