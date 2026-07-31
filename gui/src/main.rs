use std::cmp::PartialEq;
use common::networking::messages::{TcpClientMessage, TcpServerMessage};
use eframe::egui;
use egui_dock::{DockArea, DockState, TabViewer};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, LazyLock, RwLock};
use std::thread;
use common::networking::messages::UserRole;

mod controller;
mod network;
mod panels;

use crate::network::udp_client;
use crate::network::udp_client::MAX_CHANNEL;
use network::tcp_client::TcpClient;
use panels::Tab;
use crate::controller::UiEvent;
use network::connection_state;
use network::connection_state::{ConnectionState, SessionState};

pub static UI_EVENT_SENDER: LazyLock<RwLock<Option<Sender<UiEvent>>>> = LazyLock::new(||{
    RwLock::new(None)
});

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
    show_session_settings: bool,
    show_connection_settings: bool,
    server_address: String,
    username: String,
    current_theme: Theme,
    next_tab_id: u32,
    dmx_receiver: Receiver<(u8, [u8; MAX_CHANNEL])>,
    tcp_listen_receiver: Option<Receiver<TcpServerMessage>>,
    ui_event_sender: Sender<UiEvent>,
    ui_event_receiver: Receiver<UiEvent>,
    tcp_write_sender: Option<Sender<TcpClientMessage>>,
    connection_state: ConnectionState,
    session_state: SessionState,
    role: UserRole,
    password: String,
    draft_username: String,
    draft_role: UserRole,
}


impl MyApp {
    fn new(ctx: egui::Context) -> Self {
        let (dmx_sender, dmx_receiver) = mpsc::channel();

        let (ui_event_sender, ui_event_receiver) = mpsc::channel();
        
        *UI_EVENT_SENDER.write().unwrap() = Some(ui_event_sender.clone());

        // Übergibt den Context direkt an den Listener
        udp_client::start_udp_listener(None, dmx_sender, ctx)
            .expect("Failed to start UDP listener");

        // Erstelle eine neue TerminalPanel-Instanz für den initialen Tab
        let initial_terminal_panel = panels::terminal::TerminalPanel::new(0);

        Self {
            tree: DockState::new(vec![Tab::Terminal(initial_terminal_panel)]), // Hier die Instanz verwenden
            show_settings_window: false,
            show_session_settings: false,
            show_connection_settings: false,
            server_address: "127.0.0.1".to_string(),
            username: "Default User".to_string(),
            current_theme: Theme::Dark,
            next_tab_id: 1,
            dmx_receiver,
            tcp_listen_receiver: None,
            ui_event_sender,
            ui_event_receiver,
            tcp_write_sender: None,
            connection_state: ConnectionState::Disconnected,
            session_state: SessionState::LoggedOut,
            role: UserRole::Programmer,
            password: String::new(),
            draft_username: String::new(),
            draft_role: UserRole::Programmer,
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

        controller::handle_incoming_network_data(&mut self.tcp_listen_receiver, &mut self.tree, &mut self.session_state);
        controller::handle_events(&self.ui_event_receiver, &self.tcp_write_sender, &mut self.connection_state, &mut self.session_state);

        // --- TOP BAR ---
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("Connection", |ui| {
                    //if ui.button("Settings").clicked() { //TODO settings fenster machen!
                    //    self.show_settings_window = true;
                    //    ui.close_menu();
                    //}
                    if ui.button("Connection Settings").clicked() {
                        self.show_connection_settings = true;
                        ui.close_menu();
                    }
                    if ui.add_enabled(
                        matches!(self.connection_state, ConnectionState::Connected),
                        egui::Button::new("Session Settings")
                    ).clicked() {
                        self.show_session_settings = true;
                        self.draft_username = self.username.clone();
                        self.draft_role = self.role.clone();
                        self.password.clear(); // just to be safe
                        ui.close_menu();
                    }
                    if ui.add_enabled(
                        matches!(self.session_state, SessionState::LoggedIn),
                        egui::Button::new("Logout")
                    ).clicked() {
                        let event = UiEvent::LogoutRequest;
                        if let Err(e) = self.ui_event_sender.send(event) {
                            eprintln!("Failed to send UiEvent: {}", e);
                        }
                    }

                });

                ui.menu_button("Window", |ui| {
                    if ui.button("Universe").clicked() {
                        let new_tab_id = self.next_tab_id;
                        self.next_tab_id += 1;

                        let new_universe_panel = panels::universe::UniversePanel::new(new_tab_id);
                        self.tree
                            .main_surface_mut()
                            .push_to_focused_leaf(Tab::Universe(new_universe_panel));
                        ui.close_menu();
                    }
                });
            });
        });

        egui::TopBottomPanel::bottom("bottom_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let status_color = match self.connection_state {
                    ConnectionState::Disconnected | ConnectionState::Error => egui::Color32::RED,
                    ConnectionState::ConnectionPending => egui::Color32::YELLOW,
                    ConnectionState::Connected => {
                        match self.session_state {
                            SessionState::LoginFailed(_) => egui::Color32::YELLOW,
                            SessionState::LoggedIn => egui::Color32::GREEN,
                            SessionState::LoginPending | SessionState::LoggedOut => egui::Color32::YELLOW,
                        }
                    }
                };
                // Ganz links den Punkt als Vektorgrafik zeichnen (so ist er garantiert immer ein perfekter Kreis)
                let (rect, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
                ui.painter().circle_filled(rect.center(), 4.0, status_color);
                if let SessionState::LoggedIn = self.session_state {
                    ui.label(format!("{} | {}", self.username, self.role.to_string()));
                } else {
                    ui.label("Logged out");
                }

                // Rest rechtsbündig ausrichten
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!(
                        "R.E.K.T.A.L. Version: {}",
                        env!("CARGO_PKG_VERSION")
                    ));
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

        // --- CONNECTION SETTINGS POPUP ---
        let mut is_connection_open = self.show_connection_settings;
        let mut request_conn_close = false;
        
        if is_connection_open {
            egui::Window::new("Connection Settings")
                .open(&mut is_connection_open)
                .collapsible(false)
                .resizable([false, false])
                .pivot(egui::Align2::CENTER_CENTER)
                .show(ctx, |ui| {
                    egui::Grid::new("connection_seetings_grid")
                        .num_columns(2)
                        .show(ui, |ui| {
                            ui.label("Server adress ");
                            if ui.text_edit_singleline(&mut self.server_address).changed() {
                            }
                        });
                    ui.add_space(10.0);
                    
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                        if ui.button("Connect").clicked() {
                            let (tcp_write_sender, tcp_write_receiver) = mpsc::channel();
                            let (tcp_listen_sender, tcp_listen_receiver) = mpsc::channel();
                            self.tcp_write_sender = Some(tcp_write_sender);
                            self.tcp_listen_receiver = Some(tcp_listen_receiver);

                            let server_address_clone = self.server_address.clone();
                            thread::spawn(move || {
                                    let mut tcp_client = TcpClient::new(
                                        format!("{}:6767",server_address_clone).parse().unwrap(),
                                        tcp_write_receiver,
                                        tcp_listen_sender,
                                    );
                                    tcp_client.start_tcp_client();

                            });
                            request_conn_close = true;
                        }
                        if ui.button("Close").clicked() {
                            request_conn_close = true;
                        }
                    });
                });
        }
        
        if request_conn_close {
            is_connection_open = false;
        }
        self.show_connection_settings = is_connection_open;

        // --- SESSION SETTINGS POPUP ---
        let mut is_session_open = self.show_session_settings;
        let mut request_close = false;
        
        if is_session_open {
            egui::Window::new("Session Settings")
                .open(&mut is_session_open) // Fügt das 'X' zum Schließen hinzu
                .collapsible(false)
                .resizable([false, false])
                .pivot(egui::Align2::CENTER_CENTER)
                .show(ctx, |ui| {
                    egui::Grid::new("session_seetings_grid")
                        .num_columns(2)
                        .show(ui, |ui| {
                            ui.label("Username ");
                            if ui.text_edit_singleline(&mut self.draft_username).changed() {
                                //TODO connect oder save button oder ausgrauen, solang sich nix ändert!
                            }
                            ui.end_row(); // <-- Das hat gefehlt! Dadurch wird eine neue Zeile gestartet.
                            
                            ui.label("Role");
                            egui::ComboBox::from_id_source("role_combo") // from_id_source statt from_label verhindert doppelte Labels im Grid
                                .selected_text(match self.draft_role {
                                    UserRole::Programmer => "Programmer",
                                    UserRole::BlindProgrammer => "Programmer Blind",
                                    UserRole::Showrunner => "Showrunner",
                                })
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut self.draft_role, UserRole::Programmer, "Programmer");
                                    ui.selectable_value(&mut self.draft_role, UserRole::BlindProgrammer, "Programmer Blind");
                                    ui.selectable_value(&mut self.draft_role, UserRole::Showrunner, "Showrunner");
                                });
                            ui.end_row();

                            ui.label("Password");
                            let password_edit = egui::TextEdit::singleline(&mut self.password).password(true);
                            if ui.add(password_edit).changed() {
                                //TODO connect oder save button oder ausgrauen, solang sich nix ändert!
                            }
                            ui.end_row();
                        });
                    ui.add_space(10.0);

                    if let SessionState::LoginFailed(ref reason) = self.session_state {
                        ui.label(egui::RichText::new(format!("Login fehlgeschlagen: {}", reason)).color(egui::Color32::RED));
                        ui.add_space(10.0);
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                        if ui.button("Close").clicked() {
                            request_close = true;
                        }
                        if ui.button("Login").clicked() {

                            // Speichere die Änderungen aus den Entwurfs-Variablen in das eigentliche Profil
                            self.username = self.draft_username.clone();
                            self.role = self.draft_role.clone();

                            let event = UiEvent::LoginRequest {
                                user_name: self.username.clone(),
                                password: self.password.clone(),
                                user_role: self.role.clone(),
                            };
                            if let Err(e) = self.ui_event_sender.send(event) {
                                eprintln!("Failed to send UiEvent: {}", e);
                            }else{
                                request_close = true;
                            }
                            self.password.clear();
                        }
                    });
                });
        }
        
        if request_close {
            is_session_open = false;
        }

        self.show_session_settings = is_session_open;

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
