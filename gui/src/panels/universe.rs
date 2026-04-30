use eframe::egui;
use crate::network::udp_client::MAX_CHANNEL; // Use MAX_CHANNEL from udp_client

#[derive(Clone, PartialEq)] // Added Clone and PartialEq
pub struct UniversePanel {
    pub tab_id: u32, // Unique ID for this specific panel instance
    pub selected_universe: u8, // The DMX universe ID this panel is currently displaying
    pub dmx_data: [u8; MAX_CHANNEL], // The DMX data for the selected universe
}

impl UniversePanel {
    pub fn new(tab_id: u32) -> Self {
        Self {
            tab_id,
            selected_universe: 0, // Default to universe 0
            dmx_data: [0; MAX_CHANNEL],
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        let panel_id = ui.id().with(format!("universe_content_area_{}", self.tab_id));

        ui.push_id(panel_id, |ui| {
            // Quick selection buttons
            ui.label("Schnellauswahl (0 bis 15)");
            ui.horizontal_wrapped(|ui| {
                for i in 0..16 {
                    let is_selected = self.selected_universe == i;
                    if ui.selectable_label(is_selected, format!("{}", i)).clicked() {
                        self.selected_universe = i;

                        // HIER DIE MAGIE: Setze alle 512 Kanäle sofort auf 0 zurück!
                        self.dmx_data = [0; MAX_CHANNEL];

                        ui.ctx().request_repaint();
                    }
                }
            });

            ui.add_space(8.0);

            let cell_width: f32 = 50.0;
            let spacing: f32 = 4.0;
            let available_width = ui.available_width();

            let num_columns = (available_width / (cell_width + spacing)).floor() as usize;
            let num_columns = num_columns.max(1);

            let row_height = 35.0 + spacing;
            egui::ScrollArea::vertical().show_rows(ui, row_height, (MAX_CHANNEL / num_columns) + 1, |ui, row_range| {
                egui::Grid::new(format!("dmx_grid_{}", self.tab_id))
                    .num_columns(num_columns)
                    .spacing([spacing, spacing])
                    .striped(true)
                    .show(ui, |ui| {
                        for row in row_range {
                            for col in 0..num_columns {
                                let i = row * num_columns + col; // 0-indexed channel
                                if i < MAX_CHANNEL {
                                    draw_dmx_cell(ui, i + 1, self.dmx_data[i], cell_width); // Display 1-indexed channel
                                }
                            }
                            ui.end_row();
                        }
                    });
            });

            fn draw_dmx_cell(ui: &mut egui::Ui, channel_num: usize, val: u8, width: f32) {
                egui::Frame::none()
                    .fill(ui.visuals().faint_bg_color)
                    .rounding(2.0)
                    .inner_margin(2.0)
                    .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
                    .show(ui, |ui| {
                        ui.set_min_width(width);
                        ui.vertical_centered(|ui| {
                            ui.label(egui::RichText::new(channel_num.to_string()).size(10.0).weak());
                            ui.label(egui::RichText::new(val.to_string()).strong().size(14.0).monospace());
                        });
                    });
            }

            let width = ui.available_width();
            ui.label(format!("Verfügbare Breite: {}px", width));
            ui.separator();
        });
    }
}
