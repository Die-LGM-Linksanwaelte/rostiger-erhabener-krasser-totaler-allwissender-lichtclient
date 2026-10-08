//TODO: change the config-path to a os-typical one
//TODO: press enter zum laden oder speichern in popup

use eframe::egui;
use std::path::{Path, PathBuf};
use crate::controller::{send_ui_event, UiEvent};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutDialogMode {
    Load,
    SaveAs,
}

pub struct LayoutManagerWindow {
    pub is_open: bool,
    pub base_dir: PathBuf,
    pub mode: LayoutDialogMode,
    pub selected_item: Option<PathBuf>,
    pub input_name: String,

    pub show_overwrite_warning: Option<PathBuf>,
    pub renaming_path: Option<PathBuf>,
    pub rename_buffer: String,
    pub creating_folder: bool,
    pub new_folder_name: String,
    pub hovered_drop_target: Option<PathBuf>,
}

impl LayoutManagerWindow {
    pub fn new() -> Self {
        let base_dir = std::fs::canonicalize("config/window_layouts")
            .unwrap_or_else(|_| PathBuf::from("config/window_layouts"));

        Self {
            is_open: false,
            base_dir,
            mode: LayoutDialogMode::Load,
            selected_item: None,
            input_name: String::new(),
            show_overwrite_warning: None,
            renaming_path: None,
            rename_buffer: String::new(),
            creating_folder: false,
            new_folder_name: String::new(),
            hovered_drop_target: None,
        }
    }

    /// Opens the manager in Load mode.
    pub fn open_load(&mut self) {
        self.mode = LayoutDialogMode::Load;
        self.reset_dialog_state();
        self.is_open = true;
    }

    /// Opens the manager in SaveAs mode.
    pub fn open_save_as(&mut self) {
        self.mode = LayoutDialogMode::SaveAs;
        self.reset_dialog_state();
        if self.input_name.is_empty() {
            self.input_name = "new_layout".to_string();
        }
        self.is_open = true;
    }

    /// Default open method (opens in Load mode).
    pub fn open(&mut self) {
        self.open_load();
    }

    fn reset_dialog_state(&mut self) {
        self.show_overwrite_warning = None;
        self.renaming_path = None;
        self.rename_buffer.clear();
        self.creating_folder = false;
        self.new_folder_name.clear();
        self.hovered_drop_target = None;
    }

    /// Checks if a given path is the protected root `default.json` file.
    pub fn is_root_default(&self, path: &Path) -> bool {
        let root_default = self.base_dir.join("default.json");
        if let (Ok(p1), Ok(p2)) = (std::fs::canonicalize(path), std::fs::canonicalize(&root_default)) {
            p1 == p2
        } else {
            path == root_default
        }
    }

    /// Checks whether dropping `source` into `target_dir` is valid.
    pub fn is_valid_drop(&self, source: &Path, target_dir: &Path) -> bool {
        if self.is_root_default(source) || source == self.base_dir {
            return false;
        }
        if source == target_dir {
            return false;
        }
        if source.parent() == Some(target_dir) {
            return false;
        }
        if source.is_dir() && target_dir.starts_with(source) {
            return false;
        }
        true
    }

    /// Moves a file or directory into `target_dir`.
    pub fn move_item(&mut self, source: &Path, target_dir: &Path) {
        if !self.is_valid_drop(source, target_dir) {
            return;
        }
        let Some(file_name) = source.file_name() else {
            return;
        };
        let target_path = target_dir.join(file_name);

        if target_path.exists() {
            // Target already exists! Do not overwrite silently.
            return;
        }

        if let Ok(()) = std::fs::rename(source, &target_path) {
            if self.selected_item.as_deref() == Some(source) {
                self.selected_item = Some(target_path);
            }
        }
    }

    /// Determines the currently targeted directory (either the selected folder or the parent of the selected file).
    fn current_target_dir(&self) -> PathBuf {
        if let Some(item) = &self.selected_item {
            if item.is_dir() {
                item.clone()
            } else {
                item.parent().unwrap_or(&self.base_dir).to_path_buf()
            }
        } else {
            self.base_dir.clone()
        }
    }

    /// Initiates inline renaming for the given item (file or folder).
    pub fn start_rename(&mut self, path: PathBuf) {
        if self.is_root_default(&path) || path == self.base_dir {
            return;
        }
        let stem = if path.is_dir() {
            path.file_name().and_then(|s| s.to_str()).unwrap_or("")
        } else {
            path.file_stem().and_then(|s| s.to_str()).unwrap_or("")
        };
        self.rename_buffer = stem.to_string();
        self.renaming_path = Some(path);
    }

    /// Applies the rename operation on the filesystem.
    fn apply_rename(&mut self, old_path: &Path) {
        let new_name = self.rename_buffer.trim();
        if !new_name.is_empty() {
            let parent = old_path.parent().unwrap_or(&self.base_dir);
            let new_file_name = if old_path.is_dir() || new_name.ends_with(".json") {
                new_name.to_string()
            } else {
                format!("{}.json", new_name)
            };
            let new_path = parent.join(new_file_name);
            if !new_path.exists() {
                if let Ok(()) = std::fs::rename(old_path, &new_path) {
                    self.selected_item = Some(new_path);
                }
            }
        }
        self.renaming_path = None;
    }

    /// Handles save request, prompting for overwrite confirmation if file exists.
    fn attempt_save(&mut self) {
        let raw_name = self.input_name.trim();
        if raw_name.is_empty() {
            return;
        }
        let file_name = if raw_name.ends_with(".json") {
            raw_name.to_string()
        } else {
            format!("{}.json", raw_name)
        };

        let target_dir = self.current_target_dir();
        let target_path = target_dir.join(file_name);

        if target_path.exists() {
            self.show_overwrite_warning = Some(target_path);
        } else {
            self.execute_save(target_path);
        }
    }

    /// Executes the layout save via UI event.
    fn execute_save(&mut self, path: PathBuf) {
        send_ui_event(UiEvent::SaveGuiLayout { path: path.clone() });
        self.selected_item = Some(path);
        self.show_overwrite_warning = None;
        if self.mode == LayoutDialogMode::SaveAs {
            self.is_open = false;
        }
    }

    /// Renders the layout manager window and any active modals.
    pub fn show(&mut self, ctx: &egui::Context) {
        if !self.is_open {
            return;
        }

        self.hovered_drop_target = None;

        // Handle global F2 shortcut for renaming:
        if ctx.input(|i| i.key_pressed(egui::Key::F2)) {
            if let Some(selected) = self.selected_item.clone() {
                self.start_rename(selected);
            }
        }

        let mut is_open = self.is_open;
        let title = match self.mode {
            LayoutDialogMode::Load => "Layout Manager - Load Layout",
            LayoutDialogMode::SaveAs => "Layout Manager - Save Layout As",
        };

        egui::Window::new(title)
            .open(&mut is_open)
            .default_size([520.0, 420.0])
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                self.render_toolbar(ui);

                ui.separator();

                egui::ScrollArea::vertical()
                    .max_height(250.0)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        self.render_root_row(ui);

                        ui.add_space(2.0);

                        let base = self.base_dir.clone();
                        self.render_directory(ui, &base);

                        self.render_empty_space_drop_zone(ui);
                    });

                ui.separator();

                self.render_bottom_actions(ui);
            });

        self.is_open = is_open;

        self.draw_overwrite_warning(ctx);

        self.draw_dnd_preview(ctx);
    }

    fn render_toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("📁+ New Folder").clicked() {
                self.creating_folder = true;
                self.new_folder_name.clear();
            }

            let target = self.current_target_dir();
            let rel = target.strip_prefix(&self.base_dir).unwrap_or(Path::new(""));
            let rel_str = rel.to_string_lossy();
            let display_dir = if rel_str.is_empty() {
                "/".to_string()
            } else {
                format!("/{}", rel_str)
            };

            let is_root_selected = self.selected_item.as_ref() == Some(&self.base_dir);
            let root_btn = egui::Button::new(format!("📁 Target: {}", display_dir))
                .selected(is_root_selected)
                .frame(false);
            let root_response = ui.add(root_btn);

            if root_response.clicked() {
                self.selected_item = Some(self.base_dir.clone());
            }

            // Drop zone for root folder:
            if let Some(dragged) = root_response.dnd_hover_payload::<PathBuf>() {
                if self.is_valid_drop(&dragged, &self.base_dir) {
                    ui.painter().rect_filled(
                        root_response.rect.expand(2.0),
                        ui.visuals().widgets.active.rounding,
                        egui::Color32::from_rgba_unmultiplied(80, 140, 240, 60),
                    );
                    ui.painter().rect_stroke(
                        root_response.rect.expand(2.0),
                        ui.visuals().widgets.active.rounding,
                        egui::Stroke::new(2.0, egui::Color32::from_rgb(100, 170, 255)),
                    );
                }
            }

            if let Some(dragged) = root_response.dnd_release_payload::<PathBuf>() {
                let base = self.base_dir.clone();
                if self.is_valid_drop(&dragged, &base) {
                    self.move_item(&dragged, &base);
                }
            }
        });

        if self.creating_folder {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label("Name:");
                let text_edit = ui.text_edit_singleline(&mut self.new_folder_name);
                text_edit.request_focus();

                let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
                if (ui.button("Create").clicked() || enter) && !self.new_folder_name.trim().is_empty() {
                    let name = self.new_folder_name.trim();
                    let target_dir = self.current_target_dir();
                    let _ = std::fs::create_dir_all(target_dir.join(name));
                    self.creating_folder = false;
                }
                if ui.button("Cancel").clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    self.creating_folder = false;
                }
            });
        }
    }

    /// Recursive function to render directories and JSON layout files.
    fn render_directory(&mut self, ui: &mut egui::Ui, current_dir: &Path) {
        let Ok(entries) = std::fs::read_dir(current_dir) else {
            ui.label(egui::RichText::new("Couldn't read directory!").color(egui::Color32::RED));
            return;
        };

        let mut entries: Vec<_> = entries.filter_map(Result::ok).collect();
        entries.sort_by_key(|e| (!e.path().is_dir(), e.file_name()));

        for entry in entries {
            let path = entry.path();

            if path.is_dir() {
                let folder_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                let is_renaming = self.renaming_path.as_ref() == Some(&path);

                if is_renaming {
                    let mut confirm = false;
                    let mut cancel = false;

                    ui.horizontal(|ui| {
                        ui.label("📁");
                        let response = ui.add(
                            egui::TextEdit::singleline(&mut self.rename_buffer)
                                .desired_width(150.0)
                        );
                        response.request_focus();

                        if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            confirm = true;
                        }
                        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                            cancel = true;
                        }
                    });

                    if confirm {
                        self.apply_rename(&path);
                    } else if cancel {
                        self.renaming_path = None;
                    }
                } else {
                    let id = ui.make_persistent_id(&path);
                    let is_selected = self.selected_item.as_ref() == Some(&path);

                    egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, false)
                        .show_header(ui, |ui| {
                            let folder_btn = egui::Button::new(format!("📁 {}", folder_name))
                                .selected(is_selected)
                                .sense(egui::Sense::click_and_drag())
                                .frame(false);
                            let label = ui.add(folder_btn);

                            if path != self.base_dir {
                                label.dnd_set_drag_payload(path.clone());
                            }

                            // Drop target for folder
                            if let Some(dragged) = label.dnd_hover_payload::<PathBuf>() {
                                if self.is_valid_drop(&dragged, &path) {
                                    self.hovered_drop_target = Some(path.clone());
                                    ui.painter().rect_filled(
                                        label.rect.expand(2.0),
                                        ui.visuals().widgets.active.rounding,
                                        egui::Color32::from_rgba_unmultiplied(80, 140, 240, 60),
                                    );
                                    ui.painter().rect_stroke(
                                        label.rect.expand(2.0),
                                        ui.visuals().widgets.active.rounding,
                                        egui::Stroke::new(2.0, egui::Color32::from_rgb(100, 170, 255)),
                                    );
                                }
                            }

                            if let Some(dragged) = label.dnd_release_payload::<PathBuf>() {
                                if self.is_valid_drop(&dragged, &path) {
                                    self.move_item(&dragged, &path);
                                }
                            }

                            if label.clicked() {
                                self.selected_item = Some(path.clone());
                            }

                            label.context_menu(|ui| {
                                if ui.button("📁+ New Subfolder").clicked() {
                                    self.selected_item = Some(path.clone());
                                    self.creating_folder = true;
                                    self.new_folder_name.clear();
                                    ui.close_menu();
                                }
                                if ui.button("✏ Rename (F2)").clicked() {
                                    self.start_rename(path.clone());
                                    ui.close_menu();
                                }
                                if ui.button("🗑 Delete Folder").clicked() {
                                    let _ = std::fs::remove_dir_all(&path);
                                    if self.selected_item.as_ref() == Some(&path) {
                                        self.selected_item = None;
                                    }
                                    ui.close_menu();
                                }
                            });
                        })
                        .body(|ui| {
                            self.render_directory(ui, &path);
                        });
                }
            } else if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
                let display_name = path.file_stem().and_then(|n| n.to_str()).unwrap_or("");
                let is_renaming = self.renaming_path.as_ref() == Some(&path);

                if is_renaming {
                    let mut confirm = false;
                    let mut cancel = false;

                    ui.horizontal(|ui| {
                        ui.label("📄");
                        let response = ui.add(
                            egui::TextEdit::singleline(&mut self.rename_buffer)
                                .desired_width(150.0)
                        );
                        response.request_focus();

                        if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            confirm = true;
                        }
                        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                            cancel = true;
                        }
                    });

                    if confirm {
                        self.apply_rename(&path);
                    } else if cancel {
                        self.renaming_path = None;
                    }
                } else {
                    let is_selected = self.selected_item.as_ref() == Some(&path);
                    let is_default = self.is_root_default(&path);

                    let label_text = if is_default {
                        format!("{} (Default)", display_name)
                    } else {
                        format!("{}", display_name)
                    };

                    let file_btn = egui::Button::new(label_text)
                        .selected(is_selected)
                        .sense(egui::Sense::click_and_drag())
                        .frame(false);
                    let response = ui.add(file_btn);

                    if !is_default {
                        response.dnd_set_drag_payload(path.clone());
                    }

                    if response.clicked() {
                        self.selected_item = Some(path.clone());
                        if self.mode == LayoutDialogMode::SaveAs {
                            self.input_name = display_name.to_string();
                        }
                    }

                    if response.double_clicked() {
                        match self.mode {
                            LayoutDialogMode::Load => {
                                send_ui_event(UiEvent::LoadGuiLayout { path: path.clone() });
                                self.is_open = false;
                            }
                            LayoutDialogMode::SaveAs => {
                                self.input_name = display_name.to_string();
                                self.attempt_save();
                            }
                        }
                    }

                    response.context_menu(|ui| {
                        if ui.button("Load Layout").clicked() {
                            send_ui_event(UiEvent::LoadGuiLayout { path: path.clone() });
                            self.is_open = false;
                            ui.close_menu();
                        }

                        let rename_btn = ui.add_enabled(!is_default, egui::Button::new("✏ Rename (F2)"));
                        if is_default {
                            rename_btn.on_hover_text("Default layout cannot be renamed!");
                        } else if rename_btn.clicked() {
                            self.start_rename(path.clone());
                            ui.close_menu();
                        }

                        let delete_btn = ui.add_enabled(!is_default, egui::Button::new("🗑 Delete"));
                        if is_default {
                            delete_btn.on_hover_text("Default layout cannot be deleted!");
                        } else if delete_btn.clicked() {
                            let _ = std::fs::remove_file(&path);
                            if self.selected_item.as_ref() == Some(&path) {
                                self.selected_item = None;
                            }
                            ui.close_menu();
                        }
                    });
                }
            }
        }
    }

    fn render_bottom_actions(&mut self, ui: &mut egui::Ui) {
        if self.mode == LayoutDialogMode::SaveAs {
            ui.horizontal(|ui| {
                ui.label("Layout name:");
                let text_edit = ui.text_edit_singleline(&mut self.input_name);
                let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
                if (ui.button("💾 Save").clicked() || (text_edit.lost_focus() && enter))
                    && !self.input_name.trim().is_empty()
                {
                    self.attempt_save();
                }
            });
            ui.add_space(5.0);
        }

        let selected_path = self.selected_item.clone();

        if let Some(selected_path) = selected_path {
            let is_dir = selected_path.is_dir();
            let display_name = if is_dir {
                selected_path.file_name().and_then(|n| n.to_str()).unwrap_or("")
            } else {
                selected_path.file_stem().and_then(|n| n.to_str()).unwrap_or("")
            };
            let is_default = self.is_root_default(&selected_path);
            let is_base = selected_path == self.base_dir;

            ui.horizontal(|ui| {
                ui.label(format!("Selection: {}", display_name));
            });

            ui.add_space(5.0);

            ui.horizontal(|ui| {
                if !is_dir && self.mode == LayoutDialogMode::Load {
                    if ui.button("Load").clicked() {
                        send_ui_event(UiEvent::LoadGuiLayout { path: selected_path.clone() });
                        self.is_open = false;
                    }
                }

                // Delete button
                let can_delete = !is_default && !is_base;
                let delete_btn = ui.add_enabled(can_delete, egui::Button::new("🗑 Delete"));
                if is_default {
                    delete_btn.on_hover_text("The default layout cannot be deleted!");
                } else if is_base {
                    delete_btn.on_hover_text("Root directory cannot be deleted!");
                } else if delete_btn.clicked() {
                    if is_dir {
                        let _ = std::fs::remove_dir_all(&selected_path);
                    } else {
                        let _ = std::fs::remove_file(&selected_path);
                    }
                    self.selected_item = None;
                }

                // Rename button
                let can_rename = !is_default && !is_base;
                let rename_btn = ui.add_enabled(can_rename, egui::Button::new("✏ Rename (F2)"));
                if is_default {
                    rename_btn.on_hover_text("The default layout cannot be renamed!");
                } else if is_base {
                    rename_btn.on_hover_text("Root directory cannot be renamed!");
                } else if rename_btn.clicked() {
                    self.start_rename(selected_path);
                }
            });
        } else {
            ui.label(egui::RichText::new("Choose a layout or folder from the list.").weak());
        }
    }

    /// Renders the overwrite warning modal dialog if an existing file is targeted.
    fn draw_overwrite_warning(&mut self, ctx: &egui::Context) {
        let Some(overwrite_path) = self.show_overwrite_warning.clone() else {
            return;
        };

        let is_default = self.is_root_default(&overwrite_path);
        let file_stem = overwrite_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("layout")
            .to_string();

        let title = if is_default {
            "⚠️ Warning: Overwriting Default Layout"
        } else {
            "Overwrite Layout File?"
        };

        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                if is_default {
                    ui.label(
                        egui::RichText::new("⚠️ CAUTION: OVERWRITING DEFAULT LAYOUT!")
                            .color(egui::Color32::YELLOW)
                            .strong(),
                    );
                    ui.add_space(5.0);
                    ui.label("You are about to overwrite the default layout ('default.json').");
                    ui.label("This will permanently change the initial layout loaded when resetting layouts!");
                    ui.add_space(5.0);
                    ui.label("Are you sure you want to proceed?");
                } else {
                    ui.label(format!("A layout named '{}' already exists.", file_stem));
                    ui.label("Do you want to overwrite it?");
                }

                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    let confirm_text = if is_default {
                        "Overwrite Anyway"
                    } else {
                        "Overwrite"
                    };
                    if ui.button(confirm_text).clicked() {
                        self.execute_save(overwrite_path.clone());
                    }
                    if ui.button("Cancel").clicked() {
                        self.show_overwrite_warning = None;
                    }
                });
            });
    }
    /// Explicit root row at top of tree view allowing drops directly into the root folder.
    fn render_root_row(&mut self, ui: &mut egui::Ui) {
        let is_root_selected = self.selected_item.as_ref() == Some(&self.base_dir);
        let root_btn = egui::Button::new("📁 /                  ")
            .selected(is_root_selected)
            .sense(egui::Sense::click_and_drag())
            .frame(false);

        let root_res = ui.add(root_btn);

        if root_res.clicked() {
            self.selected_item = Some(self.base_dir.clone());
        }

        if let Some(dragged) = root_res.dnd_hover_payload::<PathBuf>() {
            if self.is_valid_drop(&dragged, &self.base_dir) {
                self.hovered_drop_target = Some(self.base_dir.clone());
                ui.painter().rect_filled(
                    root_res.rect.expand(2.0),
                    ui.visuals().widgets.active.rounding,
                    egui::Color32::from_rgba_unmultiplied(60, 130, 240, 70),
                );
                ui.painter().rect_stroke(
                    root_res.rect.expand(2.0),
                    ui.visuals().widgets.active.rounding,
                    egui::Stroke::new(2.0, egui::Color32::from_rgb(80, 170, 255)),
                );
            }
        }

        if let Some(dragged) = root_res.dnd_release_payload::<PathBuf>() {
            let base = self.base_dir.clone();
            if self.is_valid_drop(&dragged, &base) {
                self.move_item(&dragged, &base);
            }
        }
    }

    /// Empty space below tree view acts as a convenient drop zone to move items into root.
    fn render_empty_space_drop_zone(&mut self, ui: &mut egui::Ui) {
        let available = ui.available_size_before_wrap();
        if available.y > 25.0 {
            let (rect, empty_res) = ui.allocate_exact_size(available, egui::Sense::hover());
            if let Some(dragged) = empty_res.dnd_hover_payload::<PathBuf>() {
                if self.is_valid_drop(&dragged, &self.base_dir) {
                    self.hovered_drop_target = Some(self.base_dir.clone());
                    ui.painter().rect_filled(
                        rect,
                        4.0,
                        egui::Color32::from_rgba_unmultiplied(60, 130, 240, 40),
                    );
                    ui.painter().rect_stroke(
                        rect,
                        4.0,
                        egui::Stroke::new(1.5, egui::Color32::from_rgb(80, 170, 255)),
                    );

                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "➔ Drop here to move to Root",
                        egui::TextStyle::Body.resolve(ui.style()),
                        egui::Color32::from_rgb(180, 215, 255),
                    );
                }
            }

            if let Some(dragged) = empty_res.dnd_release_payload::<PathBuf>() {
                let base = self.base_dir.clone();
                if self.is_valid_drop(&dragged, &base) {
                    self.move_item(&dragged, &base);
                }
            }
        }
    }

    /// Renders a floating preview badge right next to the cursor while dragging an item.
    fn draw_dnd_preview(&self, ctx: &egui::Context) {
        let Some(dragged) = egui::DragAndDrop::payload::<PathBuf>(ctx) else {
            return;
        };
        let Some(pos) = ctx.pointer_latest_pos() else {
            return;
        };

        let dragged_name = dragged.file_stem().and_then(|s| s.to_str()).unwrap_or("item");
        let icon = if dragged.is_dir() { "📁" } else { "📄" };

        egui::Area::new(egui::Id::new("dnd_floating_preview"))
            .order(egui::Order::Tooltip)
            .current_pos(pos + egui::vec2(16.0, 16.0))
            .interactable(false)
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    if let Some(target) = &self.hovered_drop_target {
                        let target_name = if target == &self.base_dir {
                            "Root".to_string()
                        } else {
                            target.file_name().and_then(|n| n.to_str()).unwrap_or("folder").to_string()
                        };
                        ui.horizontal(|ui| {
                            ui.label(format!("{} {}", icon, dragged_name));
                            ui.label(
                                egui::RichText::new(format!("➔ 📁 {}", target_name))
                                    .color(egui::Color32::LIGHT_GREEN)
                                    .strong(),
                            );
                        });
                    } else {
                        ui.label(format!("{} Moving: {}", icon, dragged_name));
                    }
                });
            });
    }
}