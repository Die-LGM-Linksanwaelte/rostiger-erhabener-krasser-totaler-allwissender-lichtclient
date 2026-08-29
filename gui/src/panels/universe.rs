//! # Universe Panel Module
//!
//! This module implements the visual DMX Universe view for the R.E.K.T.A.L. GUI application.
//! It renders up to 512 DMX channels per universe in a dynamic, responsive grid layout,
//! displays live channel values, and optionally overlays patched fixture devices and property types.

use crate::network::udp_client::MAX_CHANNEL;
pub use common::fixture::PropertyType;
use common::networking::subscription_objects::{
    DMXConfigForClientState, DMXConfigurationForClient,
};
use eframe::egui;

/// UI Panel representing a single DMX universe view in the docking interface.
///
/// Displays a responsive grid of 512 DMX channels ([`MAX_CHANNEL`]), supports switching
/// between universes 1 through 16, and dynamically renders patched fixture overlays.
#[derive(Clone)]
pub struct UniversePanel {
    /// The unique ID of the tab hosting this universe panel instance.
    pub tab_id: u32,
    /// Currently selected universe index (1-based, 1..=16).
    pub selected_universe: u8,
    /// Live DMX byte values for all 512 channels of the active universe.
    pub dmx_data: [u8; MAX_CHANNEL],
    /// Configuration mapping fixtures to DMX channels received from the kernel server.
    pub device_configuration: Option<DMXConfigForClientState>,
    /// Controls whether the settings configuration window is visible.
    settings_open: bool,
    /// Minimum width of an individual DMX channel cell in pixels.
    min_pure_cell_width: f32,
    /// Height of an individual DMX channel cell in pixels.
    cell_height: f32,
    /// Flag enabling or disabling the rendering of patched fixture properties on cells.
    show_device_properties: bool,
    /// Holds the name of the fixture currently hovered by the mouse cursor.
    hovered_device: Option<String>,
}

impl UniversePanel {
    /// Creates a new [`UniversePanel`] instance with default settings for a given tab ID.
    ///
    /// # Arguments
    /// * `tab_id` - The unique identifier of the tab hosting this panel.
    pub fn new(tab_id: u32) -> Self {
        Self {
            tab_id,
            selected_universe: 1,
            dmx_data: [0; MAX_CHANNEL],
            settings_open: false,
            min_pure_cell_width: 45.0,
            cell_height: 30.0,
            show_device_properties: false,
            device_configuration: None,
            hovered_device: None,
        }
    }

    /// Renders the main universe panel UI inside an `egui` container.
    ///
    /// # Arguments
    /// * `ui` - Mutable reference to the `egui::Ui` layout context.
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        let panel_id = ui
            .id()
            .with(format!("universe_content_area_{}", self.tab_id));

        ui.push_id(panel_id, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Universe").strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("⚙").on_hover_text("Settings").clicked() {
                        self.settings_open = true;
                    }
                });
            });

            self.draw_settings_frame(ui);

            ui.horizontal_wrapped(|ui| {
                for i in 1..17 {
                    let is_selected = self.selected_universe == i;
                    if ui.selectable_label(is_selected, format!("{}", i)).clicked() {
                        if self.selected_universe != i {
                            self.selected_universe = i;
                            self.dmx_data = [0; MAX_CHANNEL]; // Clear data immediately on switch
                        }
                    }
                }
            });

            ui.add_space(8.0);

            self.draw_dmx_cell_pane(ui);
        });
    }

    /// Renders the pop-up settings window for configuring universe panel display options.
    ///
    /// # Arguments
    /// * `ui` - Mutable reference to the `egui::Ui` context.
    fn draw_settings_frame(&mut self, ui: &mut egui::Ui) {
        if self.settings_open {
            let mut is_open = self.settings_open;

            egui::Window::new("Universe Panel Settings")
                .collapsible(false)
                .resizable([false, false])
                .pivot(egui::Align2::CENTER_CENTER)
                .show(ui.ctx(), |ui| {
                    egui::Grid::new(format!("seetings_grid_{}", self.tab_id))
                        .num_columns(2)
                        .show(ui, |ui| {
                            if ui
                                .checkbox(
                                    &mut self.show_device_properties,
                                    "show devices & properties",
                                )
                                .changed()
                            {
                                self.cell_height = match self.show_device_properties {
                                    true => 60.0,
                                    false => 30.0,
                                }
                            }
                        });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                        if ui.button("Back").on_hover_text("Back").clicked() {
                            is_open = false;
                        }
                    });
                    false
                });
            self.settings_open = is_open;
        }
    }

    /// Renders the scrollable DMX channel grid and fixture overlays.
    ///
    /// Dynamically calculates the optimal number of columns based on available UI width,
    /// populates all 512 channel cells, and paints fixture overlay banners across patched channels.
    ///
    /// # Arguments
    /// * `ui` - Mutable reference to the `egui::Ui` context.
    fn draw_dmx_cell_pane(&mut self, ui: &mut egui::Ui) {
        let spacing = 4.0;
        let frame_extra = 6.0;
        let min_full_cell_width = self.min_pure_cell_width + frame_extra;

        let available_width = (ui.available_width() - 4.0).max(0.0);

        let num_columns =
            ((available_width + spacing) / (min_full_cell_width + spacing)).floor() as usize;
        let num_columns = num_columns.max(1);

        let total_spacing = num_columns.saturating_sub(1) as f32 * spacing;
        let stretched_full_cell_width = (available_width - total_spacing) / (num_columns as f32);

        let stretched_pure_cell_width = stretched_full_cell_width - frame_extra;

        let total_rows = (MAX_CHANNEL + num_columns - 1) / num_columns;
        let row_height = self.cell_height + spacing;
        let mut cell_responses = vec![None; MAX_CHANNEL];

        egui::ScrollArea::vertical()
            .id_source(format!("dmx_scroll_{}", self.tab_id))
            .auto_shrink([false, false])
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
            .show_rows(ui, row_height, total_rows, |ui, row_range| {
                egui::Grid::new(format!("dmx_grid_{}", self.tab_id))
                    .num_columns(num_columns)
                    .spacing([spacing, spacing])
                    .min_col_width(stretched_full_cell_width)
                    .max_col_width(stretched_full_cell_width)
                    .show(ui, |ui| {
                        for row in row_range {
                            for col in 0..num_columns {
                                let i = row * num_columns + col;
                                if i < MAX_CHANNEL {
                                    let dmx_config = self
                                        .device_configuration
                                        .as_ref()
                                        .and_then(|config| {
                                            config.get(self.selected_universe as usize - 1)
                                        })
                                        .and_then(|universe| universe.get(i))
                                        .cloned()
                                        .unwrap_or(DMXConfigurationForClient::Empty);

                                    let resp = self.draw_dmx_cell(
                                        ui,
                                        i + 1,
                                        dmx_config,
                                        self.dmx_data[i],
                                        stretched_pure_cell_width,
                                    );
                                    cell_responses[i] = Some(resp);
                                } else {
                                    ui.label("");
                                }
                            }
                            ui.end_row();
                        }
                    });

                let cell_rects: Vec<Option<egui::Rect>> = cell_responses
                    .iter()
                    .map(|r| r.as_ref().map(|resp| resp.rect))
                    .collect();

                // Draw device overlays inside the ScrollArea so they scroll with the content
                if self.show_device_properties {
                    if let Some(universe_config) = self
                        .device_configuration
                        .as_ref()
                        .and_then(|config| config.get(self.selected_universe as usize - 1))
                    {
                        let mut i = 0;
                        while i < MAX_CHANNEL {
                            let entry = universe_config.get(i);
                            if let Some(DMXConfigurationForClient::Reserved {
                                fixture_type_hash,
                                fixture_name,
                                ..
                            }) = entry
                            {
                                let start = i;
                                let mut end = i;
                                while end + 1 < MAX_CHANNEL {
                                    if let Some(DMXConfigurationForClient::Reserved {
                                        fixture_name: next_name,
                                        ..
                                    }) = universe_config.get(end + 1)
                                    {
                                        if next_name == fixture_name {
                                            end += 1;
                                            continue;
                                        }
                                    }
                                    break;
                                }

                                self.draw_device_overlay(
                                    ui,
                                    start,
                                    end,
                                    fixture_name.as_str(),
                                    *fixture_type_hash,
                                    &cell_rects,
                                    num_columns,
                                );
                                i = end + 1;
                            } else {
                                i += 1;
                            }
                        }
                    }

                    // Detect which device is currently hovered
                    let mut hovered_device_name = None;
                    if let Some(universe_config) = self
                        .device_configuration
                        .as_ref()
                        .and_then(|config| config.get(self.selected_universe as usize - 1))
                    {
                        for (i, resp_opt) in cell_responses.iter().enumerate() {
                            if let Some(resp) = resp_opt {
                                if resp.hovered() {
                                    if let Some(DMXConfigurationForClient::Reserved {
                                        fixture_name,
                                        ..
                                    }) = universe_config.get(i)
                                    {
                                        hovered_device_name = Some(fixture_name.clone());
                                        break;
                                    }
                                }
                            }
                        }
                    }

                    if self.hovered_device != hovered_device_name {
                        self.hovered_device = hovered_device_name;
                        ui.ctx().request_repaint();
                    }
                }
            });
    }

    /// Paints a visual banner overlay spanning across consecutive DMX channels occupied by a single fixture device.
    ///
    /// # Arguments
    /// * `ui` - Mutable reference to the `egui::Ui` context.
    /// * `start` - Starting DMX channel index (0-based).
    /// * `end` - Ending DMX channel index (0-based, inclusive).
    /// * `fixture_name` - Display name of the patched fixture device.
    /// * `fixture_type_hash` - Hash byte used to generate a unique fixture color.
    /// * `cell_rects` - Array of cell bounding rectangles for computing overlay spans.
    /// * `num_columns` - Current number of columns in the grid layout.
    fn draw_device_overlay(
        &self,
        ui: &mut egui::Ui,
        start: usize,
        end: usize,
        fixture_name: &str,
        fixture_type_hash: u8,
        cell_rects: &[Option<egui::Rect>],
        num_columns: usize,
    ) {
        let color = get_color_for_fixture_hash(fixture_type_hash);
        let border_color = color.to_opaque();

        let mut current_start = start;
        while current_start <= end {
            let row = current_start / num_columns;
            let row_end = ((row + 1) * num_columns - 1).min(end);

            let mut row_rect: Option<egui::Rect> = None;
            for c in current_start..=row_end {
                if let Some(rect) = cell_rects[c] {
                    if let Some(ref mut r) = row_rect {
                        *r = r.union(rect);
                    } else {
                        row_rect = Some(rect);
                    }
                }
            }

            if let Some(rect) = row_rect {
                let bar_height = 28.0;
                let bar_rect = egui::Rect::from_min_max(
                    egui::pos2(rect.left(), rect.top() + 13.0),
                    egui::pos2(rect.right(), rect.top() + 13.0 + bar_height),
                );

                // Highlight background and stroke if this device is hovered
                let is_hovered = Some(fixture_name) == self.hovered_device.as_deref();
                let bg_color = if is_hovered {
                    egui::Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 120)
                } else {
                    color
                };
                let stroke_width = if is_hovered { 2.0 } else { 1.0 };

                ui.painter().rect(
                    bar_rect,
                    2.0,
                    bg_color,
                    egui::Stroke::new(stroke_width, border_color),
                );

                let display_name = if current_start == start {
                    fixture_name.to_string()
                } else {
                    format!("{} (cont.)", fixture_name)
                };

                let font_size = 10.0;
                let text_pos = bar_rect.center_top() + egui::vec2(0.0, 2.0);
                ui.painter().text(
                    text_pos,
                    egui::Align2::CENTER_TOP,
                    display_name,
                    egui::FontId::monospace(font_size),
                    border_color,
                );
            }

            current_start = row_end + 1;
        }
    }

    /// Renders a single DMX channel cell inside the grid.
    ///
    /// Displays channel number, live byte value, and optional patched property details.
    ///
    /// # Arguments
    /// * `ui` - Mutable reference to the `egui::Ui` context.
    /// * `channel_num` - 1-based DMX channel number (1..=512).
    /// * `dmx_config` - Patched fixture configuration for this specific channel.
    /// * `val` - Live 8-bit DMX value (0..=255).
    /// * `width` - Target pixel width of the cell frame.
    ///
    /// # Returns
    /// An `egui::Response` representing user interaction with the cell frame.
    fn draw_dmx_cell(
        &mut self,
        ui: &mut egui::Ui,
        channel_num: usize,
        dmx_config: DMXConfigurationForClient,
        val: u8,
        width: f32,
    ) -> egui::Response {
        let is_hovered_device = if let Some(ref hovered) = self.hovered_device {
            if let DMXConfigurationForClient::Reserved { fixture_name, .. } = &dmx_config {
                fixture_name == hovered
            } else {
                false
            }
        } else {
            false
        };

        let fill_color = if is_hovered_device {
            ui.visuals().widgets.hovered.bg_fill.linear_multiply(0.5)
        } else {
            ui.visuals().faint_bg_color
        };

        let stroke = if is_hovered_device {
            egui::Stroke::new(1.5, ui.visuals().widgets.hovered.bg_stroke.color)
        } else {
            ui.visuals().widgets.noninteractive.bg_stroke
        };

        egui::Frame::none()
            .fill(fill_color)
            .rounding(2.0)
            .inner_margin(2.0)
            .stroke(stroke)
            .show(ui, |ui| {
                ui.set_width(width);
                ui.set_height(self.cell_height);
                ui.vertical_centered(|ui| {
                    // Align the channel number to the top right of the cell to avoid overlapping with the device name
                    ui.with_layout(egui::Layout::top_down(egui::Align::Max), |ui| {
                        ui.label(
                            egui::RichText::new(channel_num.to_string())
                                .size(8.0)
                                .weak(),
                        );
                    });

                    // Lays out the remaining elements from the bottom up to align them to the bottom
                    ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(val.to_string())
                                .strong()
                                .size(14.0)
                                .monospace(),
                        );
                        ui.add_space(4.0);
                        if self.show_device_properties {
                            match &dmx_config {
                                DMXConfigurationForClient::Reserved { property_type, .. } => {
                                    let prop_str = match property_type {
                                        PropertyType::Simple(simple) => format!("{:?}", simple),
                                        PropertyType::Color(color) => format!("{:?}", color),
                                    };
                                    ui.label(egui::RichText::new(prop_str).size(9.0));
                                }
                                DMXConfigurationForClient::Empty => {
                                    ui.label(egui::RichText::new("-").size(9.0).weak());
                                }
                            }
                        }
                    });
                });
            })
            .response
    }
}

/// Generates a deterministic semi-transparent [`egui::Color32`] hue based on a fixture hash byte.
///
/// # Arguments
/// * `hash` - Hash byte representing the fixture type.
///
/// # Returns
/// A semi-transparent `egui::Color32` color for rendering fixture overlay banners.
fn get_color_for_fixture_hash(hash: u8) -> egui::Color32 {
    use egui::ecolor::Hsva;
    let hue = (hash as f32) / 255.0;
    let hsva = Hsva::new(hue, 0.8, 0.8, 0.25);
    egui::Color32::from(hsva)
}
