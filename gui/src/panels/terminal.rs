use crate::controller::UiEvent;
use eframe::egui;
use eframe::egui::Color32;

use crate::UI_EVENT_SENDER;

/// A structure representing a single, colored piece of text.
#[derive(Clone)]
pub struct TextFragment {
    pub text: String,
    pub color: Color32,
}

/// The structure holding the state of the terminal panel.
#[derive(Clone)]
pub struct TerminalPanel {
    pub tab_id: u32,
    input_text: String,
    history: Vec<Vec<TextFragment>>,
    command_history: Vec<String>,
    history_length: usize,
    position_in_history: usize,
    settings_open: bool,
    settings_text: String,
}

impl TerminalPanel {
    /// Creates a new instance of the terminal panel.
    pub fn new(tab_id: u32, ) -> Self {
        let initial_line = vec![
            TextFragment {
                text: "> ".to_string(),
                color: Color32::GRAY,
            },
            TextFragment {
                text: "Konsole bereit...".to_string(), //TODO: erst printen, wenn connection established ist
                color: Color32::RED,
            },
        ];
        Self {
            tab_id,
            input_text: String::new(),
            history: vec![initial_line],
            command_history: Vec::new(),
            history_length: 100,
            position_in_history: 0,
            settings_open: false,
            settings_text: String::new(),
        }
    }

    /// Adds a multi-colored line (a collection of fragments) to the terminal history.
    pub fn add_fragments(&mut self, fragments: Vec<TextFragment>) {
        self.history.push(fragments);
        self.enforce_history_length();
    }

    /// Helper method to ensure the history does not exceed the configured maximum length.
    fn enforce_history_length(&mut self) {
        if self.history.len() > self.history_length {
            let excess = self.history.len() - self.history_length;
            self.history.drain(0..excess);
        }
    }

    /// Draws the top bar of the terminal, including the title and settings button.
    fn draw_top_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Terminal").strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("⚙").on_hover_text("Settings").clicked() {
                    self.settings_open = true;
                    self.settings_text = self.history_length.to_string();
                }
            });
        });
    }

    /// Draws the settiui_event_senderngs window if it is currently open.
    fn draw_settings_window(&mut self, ui: &mut egui::Ui) {
        if self.settings_open {
            let mut is_open = self.settings_open;

            egui::Window::new(format!("Settings - Panel {}", self.tab_id))
                .collapsible(false)
                .resizable([false, false])
                .pivot(egui::Align2::CENTER_CENTER)
                .show(ui.ctx(), |ui| {
                    egui::Grid::new(format!("seetings_grid_{}", self.tab_id))
                        .num_columns(2)
                        .show(ui, |ui| {
                            ui.label("Command history lenght: ");
                            if ui.text_edit_singleline(&mut self.settings_text).changed() {
                                // Nur positive Ganzzahlen zulassen (Ziffern)
                                self.settings_text.retain(|c| c.is_ascii_digit());
                            }
                        });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                        if ui.button("save").on_hover_text("Save").clicked() {
                            if let Ok(val) = self.settings_text.parse::<usize>() {
                                self.history_length = val;
                                self.enforce_history_length();
                            }
                            is_open = false;
                        }
                        if ui.button("discard").on_hover_text("Discard").clicked() {
                            is_open = false;
                        }
                    });
                    false
                });
            self.settings_open = is_open;
        }
    }

    /// Draws the central panel containing the scrollable terminal history.
    fn draw_central_panel(&mut self, ui: &mut egui::Ui) {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            egui::ScrollArea::vertical()
                .stick_to_bottom(true)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    // Iteriere durch die Zeilen in der History
                    for line_fragments in &self.history {
                        // Jede Zeile wird in einem horizontalen Layout dargestellt
                        ui.horizontal_wrapped(|ui| {
                            // Iteriere durch die farbigen Fragmente in der Zeile
                            for fragment in line_fragments {
                                ui.label(egui::RichText::new(&fragment.text).color(fragment.color));
                            }
                        });
                    }
                });
        });
    }

    /// Draws the bottom input panel where users can enter commands.
    fn draw_input_panel(&mut self, ui: &mut egui::Ui) {
        egui::TopBottomPanel::bottom("terminal_input_panel")
            .resizable(false)
            .show_inside(ui, |ui| {
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("> ");
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.input_text)
                            .desired_width(f32::INFINITY),
                    );

                    if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        if !self.input_text.is_empty() {
                            // Füge den eingegebenen Befehl als mehrfarbige Zeile zur History hinzu
                            self.add_fragments(vec![
                                TextFragment {
                                    text: "> ".to_string(),
                                    color: Color32::GRAY,
                                },
                                TextFragment {
                                    text: self.input_text.clone(),
                                    color: Color32::WHITE,
                                },
                            ]);
                            
                            self.command_history.push(self.input_text.clone());
                            self.position_in_history = 0; // Reset history position

                            if let Some(sender) = UI_EVENT_SENDER.read().unwrap().as_ref() {
                                if let Err(e) = sender.send(UiEvent::SendTerminalCommand {
                                    id: self.tab_id,
                                    command: self.input_text.clone(),
                                }) {
                                    eprintln!("Failed to send UiEvent: {}", e);
                                }
                            }
                            self.input_text.clear();
                            response.request_focus();
                        }
                    }
                    if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                        if !self.command_history.is_empty() {
                            if self.position_in_history < self.command_history.len() {
                                self.position_in_history += 1;
                            }
                            self.input_text = self.command_history
                                [self.command_history.len() - self.position_in_history]
                                .clone();
                        }
                    }

                    if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                        if self.position_in_history > 1 {
                            self.position_in_history -= 1;
                            self.input_text = self.command_history
                                [self.command_history.len() - self.position_in_history]
                                .clone();
                        } else if self.position_in_history == 1 {
                            self.position_in_history = 0;
                            self.input_text.clear();
                        }
                    }
                });
            });
    }

    /// Renders the UI logic for the terminal panel.
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        self.draw_top_bar(ui);
        self.draw_settings_window(ui);
        self.draw_central_panel(ui);
        self.draw_input_panel(ui);
    }
}
