use eframe::egui;
use egui_dock::{DockArea, DockState, NodeIndex, TabViewer};

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 768.0])
            .with_title("Rust Professional Docking UI"),
        ..Default::default()
    };

    eframe::run_native(
        "Docking App",
        options,
        Box::new(|_cc| Ok(Box::new(MyApp::new()))),
    )
}

// --- DATENSTRUKTUR ---

struct MyApp {
    tree: DockState<String>,
}

impl MyApp {
    fn new() -> Self {
        // Initiales Layout definieren
        let mut tree = DockState::new(vec!["Hauptfeld".to_owned()]);
        let surface = tree.main_surface_mut();
        let root = NodeIndex::root();

        // Den Bildschirm zerschneiden (Docking-Nodes)
        let [main_node, _right_node] = surface.split_right(root, 0.78, vec!["Eigenschaften".to_owned()]);
        let [main_node, _bottom_node] = surface.split_below(main_node, 0.7, vec!["Terminal".to_owned(), "Ausgabe".to_owned()]);
        surface.split_left(main_node, 0.2, vec!["Dateibrowser".to_owned()]);

        Self { tree }
    }
}

// --- UI LOGIK ---

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {

        // --- THEME INITIALISIERUNG ---
        // Wir holen die aktuellen Visuals, passen die Rundungen an
        // und schicken sie nur zurück, wenn sie noch nicht gesetzt waren.
        let mut visuals = ctx.style().visuals.clone();
        if visuals.window_rounding != 8.0.into() {
            visuals.window_rounding = 8.0.into();
            visuals.widgets.noninteractive.rounding = 8.0.into();
            ctx.set_visuals(visuals);
        }

        // --- DIE MENÜLEISTE OBEN ---
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("Datei", |ui| {
                    if ui.button("Beenden").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });

                // Theme-Umschalter auf der rechten Seite
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // WICHTIG: Wir setzen hier explizit neue Visuals-Instanzen
                    if ui.button("🌙").clicked() {
                        ctx.set_visuals(egui::Visuals::dark());
                    }
                    if ui.button("☀️").clicked() {
                        ctx.set_visuals(egui::Visuals::light());
                    }
                });
            });
        });

        // --- DAS DOCKING SYSTEM ---
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut tab_viewer = MyTabViewer {};
            DockArea::new(&mut self.tree)
                // Diese Zeile sorgt dafür, dass das Docking-System
                // die Farben vom aktuellen UI-Style übernimmt
                .style(egui_dock::Style::from_egui(ui.style().as_ref()))
                .show_inside(ui, &mut tab_viewer);
        });
    }
}

// --- TAB INHALTE ---

struct MyTabViewer;

impl TabViewer for MyTabViewer {
    type Tab = String;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        tab.clone().into()
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        match tab.as_str() {
            "Hauptfeld" => {
                ui.vertical_centered(|ui| {
                    ui.add_space(20.0);
                    ui.heading("🚀 Rust Editor");
                    ui.label("Dieses Fenster ist frei verschiebbar.");
                });
                ui.separator();
                ui.add(egui::TextEdit::multiline(&mut "Hier könnte dein Code stehen...").font(egui::TextStyle::Monospace));
            }
            "Dateibrowser" => {
                ui.collapsing("📁 project", |ui| {
                    ui.label("📄 main.rs");
                    ui.label("📄 Cargo.toml");
                });
            }
            "Eigenschaften" => {
                ui.strong("Widget-Details");
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.text_edit_singleline(&mut "Objekt_1");
                });
            }
            "Terminal" => {
                ui.label("Terminal-Ausgabe:");
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.monospace("> cargo run --example docking");
                    ui.monospace("Compiling...");
                    ui.monospace("Success!");
                });
            }
            _ => {
                ui.label(format!("Inhalt für {}", tab));
            }
        }
    }
}