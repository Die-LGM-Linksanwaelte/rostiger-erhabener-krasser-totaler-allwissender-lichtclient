//! # Terminal Panel Module
//!
//! This module implements the interactive Terminal UI panel for the R.E.K.T.A.L. GUI application.
//! It displays a scrollable log history of colored text fragments ([`TextFragment`]), supports command 
//! history navigation using the arrow keys, and dispatches user commands to the central controller.

use crate::controller::UiEvent;
use crate::UI_EVENT_SENDER;
use common::logging::LogLevel::*;
use common::r_log;
use eframe::egui;
use eframe::egui::Color32;

/// Represents a single piece of text with an associated display color.
#[derive(Clone)]
pub struct TextFragment {
    /// The string content of the fragment.
    pub text: String,
    /// The color used to render this fragment in the terminal log output.
    pub color: Color32,
}

/// UI Panel managing the interactive terminal tab state and rendering.
#[derive(Clone)]
pub struct TerminalPanel {
    /// Unique tab identifier hosting this terminal panel instance.
    pub tab_id: u32,
    /// Current input text entered in the terminal command line.
    input_text: String,
    /// History of output lines, where each line consists of multiple [`TextFragment`]s.
    history: Vec<Vec<TextFragment>>,
    /// Record of previously executed commands for arrow key navigation.
    command_history: Vec<String>,
    /// Maximum number of output lines retained in history.
    history_length: usize,
    /// Pointer index for navigating through `command_history`.
    position_in_history: usize,
    /// Visibility state of the terminal panel settings window.
    settings_open: bool,
    /// Temporary text edit buffer for settings inputs.
    settings_text: String,
    /// Indicates whether the terminal is active and accepting user input (e.g. when authenticated).
    pub(crate) is_active: bool,
}

impl TerminalPanel {
    /// Creates a new [`TerminalPanel`] instance for the specified tab ID.
    ///
    /// # Arguments
    /// * `tab_id` - Unique identifier of the host tab.
    pub fn new(tab_id: u32) -> Self {
        let initial_line = vec![];
        Self {
            tab_id,
            input_text: String::new(),
            history: vec![initial_line],
            command_history: Vec::new(),
            history_length: 100,
            position_in_history: 0,
            settings_open: false,
            settings_text: String::new(),
            is_active: false,
        }
    }

    /// Appends a multi-colored line composed of [`TextFragment`]s to the terminal output history.
    ///
    /// # Arguments
    /// * `fragments` - A vector of colored text fragments representing one line of log output.
    pub fn add_fragments(&mut self, fragments: Vec<TextFragment>) {
        self.history.push(fragments);
        self.enforce_history_length();
    }

    /// Truncates the terminal output history if it exceeds `history_length`.
    fn enforce_history_length(&mut self) {
        if self.history.len() > self.history_length {
            let excess = self.history.len() - self.history_length;
            self.history.drain(0..excess);
        }
    }

    /// Renders the top bar of the terminal panel, including the settings gear button.
    ///
    /// # Arguments
    /// * `ui` - Mutable reference to the `egui::Ui` context.
    fn draw_top_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("⚙").on_hover_text("Settings").clicked() {
                    self.settings_open = true;
                    self.settings_text = self.history_length.to_string();
                }
            });
        });
    }

    /// Renders the modal settings window for configuring maximum history length.
    ///
    /// # Arguments
    /// * `ui` - Mutable reference to the `egui::Ui` context.
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

    /// Renders the central scrollable area containing formatted log history lines.
    ///
    /// Automatically scrolls and sticks to the bottom when new output lines arrive.
    ///
    /// # Arguments
    /// * `ui` - Mutable reference to the `egui::Ui` context.
    fn draw_central_panel(&mut self, ui: &mut egui::Ui) {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            egui::ScrollArea::vertical()
                .stick_to_bottom(true)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for line_fragments in &self.history {
                        ui.horizontal_wrapped(|ui| {
                            for fragment in line_fragments {
                                ui.label(egui::RichText::new(&fragment.text).color(fragment.color));
                            }
                        });
                    }
                });
        });
    }

    /// Renders the bottom command prompt input field.
    ///
    /// Dispatches [`UiEvent::SendTerminalCommand`] on Enter press and handles Up/Down arrow key history navigation.
    ///
    /// # Arguments
    /// * `ui` - Mutable reference to the `egui::Ui` context.
    fn draw_input_panel(&mut self, ui: &mut egui::Ui) {
        egui::TopBottomPanel::bottom("terminal_input_panel")
            .resizable(false)
            .show_inside(ui, |ui| {
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("> ");
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.input_text)
                            .desired_width(f32::INFINITY)
                            .interactive(self.is_active),
                    );

                    if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        if !self.input_text.is_empty() {
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
                                    r_log!(Error, "Failed to send UiEvent: {}", e);
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

    /// Main entry point to render the entire terminal panel layout.
    ///
    /// # Arguments
    /// * `ui` - Mutable reference to the `egui::Ui` context.
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        self.draw_settings_window(ui);
        self.draw_central_panel(ui);
        self.draw_input_panel(ui);
        self.draw_top_bar(ui);
    }
}
