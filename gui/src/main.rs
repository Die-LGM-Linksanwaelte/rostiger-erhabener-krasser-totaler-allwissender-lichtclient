use eframe::egui;
use egui_dock::{DockArea, DockState, NodeIndex, TabViewer};
use std::collections::HashMap;

mod panels;
use panels::Tab;

// Importe aus fixture_test für Test-Daten
use FixtureTest::fixture::{Fixture, FixtureType, FIXTURE_LIST};

fn main() -> eframe::Result<()> {
    // 1. Test-Daten anlegen, damit die GUI nicht leer ist
    setup_test_data();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 768.0])
            .with_title("R.E.K.T.A.L. Control Center"),
        ..Default::default()
    };

    eframe::run_native(
        "Docking App",
        options,
        Box::new(|_cc| Ok(Box::new(MyApp::new()))),
    )
}

fn setup_test_data() {
    let mut list = FIXTURE_LIST.write().unwrap();
    
    // Einfacher Dimmer-Typ
    let mut props = HashMap::new();
    props.insert("dimmer".to_string(), (1, None));
    let dimmer_type = FixtureType::new("Einfacher Dimmer".to_string(), props).unwrap();
    list.fixture_types.insert("dimmer".to_string(), dimmer_type);

    // Moving Head Typ
    let mut mh_props = HashMap::new();
    mh_props.insert("dimmer".to_string(), (1, None));
    mh_props.insert("pan".to_string(), (2, Some(3)));
    mh_props.insert("tilt".to_string(), (4, Some(5)));
    let mh_type = FixtureType::new("Moving Head".to_string(), mh_props).unwrap();
    list.fixture_types.insert("moving_head".to_string(), mh_type);

    // Ein paar Instanzen erzeugen
    let spot1 = Fixture::new(list.fixture_types.get("moving_head").unwrap(), 1, 0, "Spot 1".to_string()).unwrap();
    list.fixtures.insert("Spot 1".to_string(), spot1);
    
    let par1 = Fixture::new(list.fixture_types.get("dimmer").unwrap(), 10, 0, "PAR 1".to_string()).unwrap();
    list.fixtures.insert("PAR 1".to_string(), par1);
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
        
        surface.split_left(root, 0.3, vec![Tab::Fixture("Spot 1".to_string())]);
        surface.split_right(root, 0.3, vec![Tab::Fixture("PAR 1".to_string())]);

        Self { tree }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("Projekt", |ui| {
                    if ui.button("Beenden").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });

                ui.menu_button("Hinzufügen", |ui| {
                    let list = FIXTURE_LIST.read().unwrap();
                    for name in list.fixtures.keys() {
                        if ui.button(format!("🔦 {}", name)).clicked() {
                            self.tree.main_surface_mut().push_to_focused_leaf(Tab::Fixture(name.clone()));
                            ui.close_menu();
                        }
                    }
                    ui.separator();
                    if ui.button("🌌 Universe 0 Übersicht").clicked() {
                        self.tree.main_surface_mut().push_to_focused_leaf(Tab::Universe(0));
                        ui.close_menu();
                    }
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("🌙").clicked() {
                        ctx.set_visuals(egui::Visuals::dark());
                    }
                    if ui.button("☀️").clicked() {
                        ctx.set_visuals(egui::Visuals::light());
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
