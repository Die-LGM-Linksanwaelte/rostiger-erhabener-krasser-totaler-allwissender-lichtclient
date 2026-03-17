use eframe::egui;
use egui_dock::{DockArea, DockState, NodeIndex, TabViewer};
use std::collections::HashMap;

mod panels;
use panels::Tab;

// Importe aus fixture_test für Test-Daten
use FixtureTest::fixture::{Fixture, FixtureType, FIXTURE_LIST};

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

struct MyApp {
    tree: DockState<Tab>,
}

impl MyApp {
    fn new() -> Self {
        // Start-Layout: Terminal und zwei Fixture-Panels
        let mut tree = DockState::new(vec![Tab::Terminal]);
        let surface = tree.main_surface_mut();
        let root = NodeIndex::root();

        Self { tree }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("Projekt", |ui| {
                    if ui.button("import Project").clicked() {
                        //ctx.send_viewport_cmd(egui::ViewportCommand::Close); TODO: change for importing projects
                    }
                    if ui.button("export Project").clicked() {
                        //ctx.send_viewport_cmd(egui::ViewportCommand::Close); TODO: change for exporting projects
                    }
                    if ui.button("Settings").clicked() {
                        //ctx.send_viewport_cmd(egui::ViewportCommand::Close); TODO: logic implementation
                    }
                    if ui.button("close project").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });

                ui.menu_button("Window", |ui| {
                    ui.separator();
                    if ui.button("Universe").clicked() {
                        self.tree
                            .main_surface_mut()
                            .push_to_focused_leaf(Tab::Universe(0));
                        ui.close_menu();
                    }
                });

                /*ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("🌙").clicked() {
                        ctx.set_visuals(egui::Visuals::dark());
                    }
                    if ui.button("☀️").clicked() {
                        ctx.set_visuals(egui::Visuals::light());
                    }
                });*///TODO: move the a settings panel
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            let mut tab_viewer = MyTabViewer {};
            DockArea::new(&mut self.tree)
                .style(egui_dock::Style::from_egui(ui.style().as_ref()))
                .show_inside(ui, &mut tab_viewer);
        });
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
