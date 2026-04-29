use eframe::egui;
use egui_dock::{DockArea, DockState, TabViewer};

mod panels;
use panels::Tab;
use crate::panels::universe::UniversePanel;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 768.0])
            .with_title("R.E.K.T.A.L."),
        ..Default::default()
    };

    eframe::run_native(
        "Docking App",
        options,
        Box::new(|_cc| Ok(Box::new(MyApp::new()))),
    )
}

#[derive(PartialEq, Debug, Clone, Copy)]
enum Theme { Dark, Light }

struct MyApp {
    tree: DockState<Tab>,
    show_settings_window: bool,
    username: String,
    current_theme: Theme,
    pub next_tab_id: u32,
}

impl MyApp {
    fn new() -> Self {
        Self {
            tree: DockState::new(vec![Tab::Terminal]),
            show_settings_window: false,
            username: "Default User".to_string(),
            current_theme: Theme::Dark,
            next_tab_id: 1, // Start bei 1
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // --- TOP BAR ---
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("Projekt", |ui| {
                    if ui.button("Settings").clicked() {
                        self.show_settings_window = true;
                        ui.close_menu();
                    }
                });

                ui.menu_button("Window", |ui| {
                    if ui.button("Universe").clicked() {
                        let new_id = self.next_tab_id;
                        self.next_tab_id += 1;

                        self.tree.main_surface_mut().push_to_focused_leaf(Tab::Universe {
                            tab_id: new_id,
                            selected_universe: 1,
                        });
                        ui.close_menu();
                    }
                });
            });
        });

        // --- DOCKING AREA ---
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut tab_viewer = MyTabViewer {};
            DockArea::new(&mut self.tree)
                .style(egui_dock::Style::from_egui(ui.style().as_ref()))
                .show_inside(ui, &mut tab_viewer);
        });

        // --- SETTINGS WINDOW (VIEWPORT) ---
        if self.show_settings_window {
            ctx.show_viewport_immediate(
                egui::ViewportId::from_hash_of("settings_id"),
                egui::ViewportBuilder::default()
                    .with_title("Settings")
                    .with_inner_size([450.0, 350.0]),
                |ctx, _| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        egui::Grid::new("settings_grid").num_columns(2).show(ui, |ui| {
                            ui.label("Name:");
                            ui.text_edit_singleline(&mut self.username);
                            ui.end_row();

                            ui.label("Theme:");
                            egui::ComboBox::from_id_source("theme_combo")
                                .selected_text(format!("{:?}", self.current_theme))
                                .show_ui(ui, |ui| {
                                    if ui.selectable_value(&mut self.current_theme, Theme::Dark, "Dark").clicked() {
                                        ctx.set_visuals(egui::Visuals::dark());
                                    }
                                    if ui.selectable_value(&mut self.current_theme, Theme::Light, "Light").clicked() {
                                        ctx.set_visuals(egui::Visuals::light());
                                    }
                                });
                            ui.end_row();
                        });
                    });
                },
            );

            // Handle Closing
            if ctx.input(|i| i.viewport().close_requested()) {
                self.show_settings_window = false;
            }
        }
    }
}

// --- TAB VIEWER ---
struct MyTabViewer;

impl TabViewer for MyTabViewer {
    type Tab = Tab;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        match tab {
            Tab::Terminal => "Terminal".into(),
            Tab::Universe { selected_universe, .. } => format!("Universum {}", selected_universe).into(),
            _ => {String::from("penis").into()}
        }
    }

    // --- DIESE FUNKTION FEHLT WAHRSCHEINLICH ODER IST FALSCH ---
    fn id(&mut self, tab: &mut Self::Tab) -> egui::Id {
        match tab {
            Tab::Terminal => egui::Id::new("terminal_unique"),
            // Wir nutzen die tab_id als Basis für die EGUI ID
            Tab::Universe { tab_id, .. } => egui::Id::new("uni_tab").with(tab_id),
            _ => {String::from("penis").into()}
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        match tab {
            Tab::Universe { selected_universe, .. } => {
                // Hier rufen wir dein Panel auf
                UniversePanel::ui(ui, selected_universe, 10);
            }
            Tab::Terminal => { ui.label("Terminal"); }
            _ => {}
        }
    }
}