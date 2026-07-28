use common::networking::messages::TcpServerMessage;
use eframe::egui;
use egui_dock::{DockArea, DockState, TabViewer};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, LazyLock, RwLock};
use std::thread;

mod controller;
mod network;
mod panels;

use crate::network::udp_client;
use crate::network::udp_client::MAX_CHANNEL;
use network::tcp_client::TcpClient;
use panels::Tab;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 768.0])
            .with_title("R.E.K.T.A.L."),
        ..Default::default()
    };

    //let tcp_client = network::tcp_client("bla", 1234);

    eframe::run_native(
        "Docking App",
        options,
        Box::new(|cc| Ok(Box::new(MyApp::new(cc.egui_ctx.clone())))),
    )
}

#[derive(PartialEq, Debug, Clone, Copy)]
enum Theme {
    Dark,
    Light,
}

pub struct MyApp {
    tree: DockState<Tab>,
    show_settings_window: bool,
    username: String,
    current_theme: Theme,
    next_tab_id: u32,
    dmx_receiver: Receiver<(u8, [u8; MAX_CHANNEL])>,
    tcp_listen_receiver: Receiver<TcpServerMessage>,
    ui_event_sender: Sender<controller::UiEvent>,
    ui_event_receiver: Receiver<controller::UiEvent>,
    tcp_write_sender: Sender<common::networking::messages::TcpClientMessage>,
}

impl MyApp {
    // Nimmt jetzt den Context entgegen
    fn new(ctx: egui::Context) -> Self {
        let (dmx_sender, dmx_receiver) = mpsc::channel();
        let (tcp_write_sender, tcp_write_receiver) = mpsc::channel();
        let (tcp_listen_sender, tcp_listen_receiver) = mpsc::channel();
        let (ui_event_sender, ui_event_receiver) = mpsc::channel();

        // Übergibt den Context direkt an den Listener
        udp_client::start_udp_listener(None, dmx_sender, ctx)
            .expect("Failed to start UDP listener");

        // Erstelle eine neue TerminalPanel-Instanz für den initialen Tab
        let initial_terminal_panel = panels::terminal::TerminalPanel::new(0, ui_event_sender.clone());

        let mut tcp_client = TcpClient::new(
            "127.0.0.1:6767".parse().unwrap(),
            tcp_write_receiver,
            tcp_listen_sender,
        );
        thread::spawn(move || {
            tcp_client.start_tcp_client();
        });

        Self {
            tree: DockState::new(vec![Tab::Terminal(initial_terminal_panel)]), // Hier die Instanz verwenden
            show_settings_window: false,
            username: "Default User".to_string(),
            current_theme: Theme::Dark,
            next_tab_id: 1,
            dmx_receiver,
            tcp_listen_receiver,
            ui_event_sender,
            ui_event_receiver,
            tcp_write_sender,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // --- DMX-Daten aus dem Puffer abholen ---
        // (Der Context wurde bereits vom UDP-Thread geweckt, daher läuft diese Funktion jetzt)
        while let Ok((universe_id, dmx_data)) = self.dmx_receiver.try_recv() {
            for (_, tab) in self.tree.iter_all_tabs_mut() {
                if let Tab::Universe(panel) = tab {
                    if panel.selected_universe - 1 == universe_id {
                        panel.dmx_data.copy_from_slice(&dmx_data);
                    }
                }
            }
        }

        controller::handle_incoming_network_data(&mut self.tcp_listen_receiver, &mut self.tree);
        controller::handle_outgoing_events(&self.ui_event_receiver, &self.tcp_write_sender);

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
                        let new_tab_id = self.next_tab_id;
                        self.next_tab_id += 1;

                        let new_universe_panel = panels::universe::UniversePanel::new(new_tab_id, self.ui_event_sender.clone());
                        self.tree
                            .main_surface_mut()
                            .push_to_focused_leaf(Tab::Universe(new_universe_panel));
                        ui.close_menu();
                    }
                });
            });
        });

        egui::TopBottomPanel::bottom("bottom_bar").show(ctx, |ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                ui.label(format!(
                    "R.E.K.T.A.L. Version: {}",
                    env!("CARGO_PKG_VERSION").to_string()
                ));
            });
        });

        // --- DOCKING AREA ---
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut tab_viewer = MyTabViewer {};
            DockArea::new(&mut self.tree)
                .style(egui_dock::Style::from_egui(ui.style().as_ref()))
                .show_inside(ui, &mut tab_viewer);
        });

        // --- SETTINGS WINDOW ---
        if self.show_settings_window {
            //TODO: verschieben in eine eigene datei.
            ctx.show_viewport_immediate(
                egui::ViewportId::from_hash_of("settings_id"),
                egui::ViewportBuilder::default()
                    .with_title("Settings")
                    .with_inner_size([450.0, 350.0]),
                |ctx, _| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        egui::Grid::new("settings_grid")
                            .num_columns(2)
                            .show(ui, |ui| {
                                ui.label("Name:");
                                ui.text_edit_singleline(&mut self.username);
                                ui.end_row();

                                ui.label("Theme:");
                                egui::ComboBox::from_id_source("theme_combo")
                                    .selected_text(format!("{:?}", self.current_theme))
                                    .show_ui(ui, |ui| {
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
                    });
                },
            );

            if ctx.input(|i| i.viewport().close_requested()) {
                //TODO: fenster schließt sich nicht. button funktioniert nicht!
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
        tab.title().into()
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        tab.ui(ui);
    }

    fn id(&mut self, tab: &mut Self::Tab) -> egui::Id {
        egui::Id::new(tab.unique_id())
    }
}
