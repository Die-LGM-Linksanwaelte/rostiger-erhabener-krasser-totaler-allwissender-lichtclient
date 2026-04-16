use eframe::egui;
use egui_dock::{DockArea, DockState, TabViewer};
// NodeIndex und HashMap wurden entfernt, da ungenutzt

mod panels;
use panels::Tab;

#[allow(unused_imports)]
use common::fixture::{Fixture, FixtureType, FIXTURE_LIST};

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
enum Theme {
    Dark,
    Light,
}

struct MyApp {
    tree: DockState<Tab>,
    show_settings_window: bool,

    // HIER: Deine neuen Variablen hinzufügen!
    username: String,
    current_theme: Theme,
}

impl MyApp {
    fn new() -> Self {
        Self {
            tree: DockState::new(vec![Tab::Terminal]),
            show_settings_window: false,

            // HIER: Die Startwerte festlegen!
            username: "Default User".to_string(),
            current_theme: Theme::Dark,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("Projekt", |ui| {
                    if ui.button("Import Project").clicked() {
                        ui.close_menu();
                    }
                    if ui.button("Export Project").clicked() {
                        ui.close_menu();
                    }

                    ui.separator();

                    if ui.button("Settings").clicked() {
                        self.show_settings_window = true;
                        ui.close_menu();
                    }

                    if ui.button("Close Project").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });

                ui.menu_button("Window", |ui| {
                    if ui.button("Universe").clicked() {
                        self.tree
                            .main_surface_mut()
                            .push_to_focused_leaf(Tab::Universe(0));
                        ui.close_menu();
                    }
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            let mut tab_viewer = MyTabViewer {};
            DockArea::new(&mut self.tree)
                .style(egui_dock::Style::from_egui(ui.style().as_ref()))
                .show_inside(ui, &mut tab_viewer);
        });

        if self.show_settings_window {
            // Wir fragen ab, ob der Viewport für die Einstellungen existiert.
            // show_viewport_immediate gibt uns die Möglichkeit, auf Events zu reagieren.
            ctx.show_viewport_immediate(
                egui::ViewportId::from_hash_of("settings_id"),
                egui::ViewportBuilder::default()
                    .with_title("R.E.K.T.A.L. Settings")
                    .with_inner_size([450.0, 350.0]),
                |ctx, class| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        egui::Grid::new("mein_einzigartiges_grid_id") // Jedes Grid braucht eine ID
                            .num_columns(2) // Wir wollen 2 Spalten
                            .spacing([40.0, 10.0]) // [horizontaler, vertikaler] Abstand
                            .show(ui, |ui| {
                                // --- ZEILE 1 ---
                                ui.label("Name:");
                                ui.text_edit_singleline(&mut self.username);
                                ui.end_row(); // WICHTIG: Springt in die nächste Zeile

                                // --- ZEILE 2 ---
                                ui.label("Theme:");
                                egui::ComboBox::from_label("Design")
                                    .selected_text(match self.current_theme {
                                        Theme::Dark => "Dark",
                                        Theme::Light => "Light",
                                    })
                                    .show_ui(ui, |ui| {
                                        // Option 1: Dark Mode
                                        if ui
                                            .selectable_value(
                                                &mut self.current_theme,
                                                Theme::Dark,
                                                "Dark",
                                            )
                                            .clicked()
                                        {
                                            ctx.set_visuals(egui::Visuals::dark());
                                        }

                                        // Option 2: Light Mode
                                        if ui
                                            .selectable_value(
                                                &mut self.current_theme,
                                                Theme::Light,
                                                "Light",
                                            )
                                            .clicked()
                                        {
                                            ctx.set_visuals(egui::Visuals::light());
                                        }
                                    });
                                ui.end_row();
                            });
                        ui.add_space(20.0);
                    });

                    if ctx.input(|i| i.viewport().close_requested()) {
                        // TODO: Einstellungen in Datei speichern
                        todo!("Einstellungen in Datei speichern");
                    }
                },
            );

            // Wir prüfen im Kontext des HAUPTFENSTERS, ob das Einstellungsfenster
            // gerade ein Schließ-Event gesendet hat.
            let is_closing = ctx.input(|i| {
                i.raw
                    .viewports
                    .get(&egui::ViewportId::from_hash_of("settings_id"))
                    .map_or(false, |v: &egui::ViewportInfo| v.close_requested())
            });

            if is_closing {
                self.show_settings_window = false;
            }
        }
    }
}

struct MyTabViewer;

impl TabViewer for MyTabViewer {
    type Tab = Tab;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        tab.title().into()
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        tab.ui(ui);
    }
}
