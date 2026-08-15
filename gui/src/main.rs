use common::logging::LogLevel::*;
use common::logging::{FileSink, Logger, TerminalSink};
use common::networking::messages::UserRole;
use common::networking::messages::{TcpClientMessage, TcpServerMessage};
use common::r_log;
use eframe::egui;

use egui_dock::{DockArea, DockState, TabViewer};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, LazyLock, RwLock};
use common::networking::subscription_objects::SubscribeTopic::DMXConfiguration;
use crate::controller::{send_ui_event, UiEvent};
use crate::network::udp_client;
use crate::network::udp_client::MAX_CHANNEL;
use network::connection_state::{ConnectionState, SessionState};
use network::tcp_client::TcpClient;
use panels::Tab;
use crate::network::connection_state::ConnectionState::Connected;

mod controller;
mod network;
mod panels;

pub static UI_EVENT_SENDER: LazyLock<RwLock<Option<Sender<UiEvent>>>> =
    LazyLock::new(|| RwLock::new(None));

fn main() -> eframe::Result<()> {
    let log_path = std::env::temp_dir().join("rektal_gui.log");
    Logger::global().add_sink(Box::new(FileSink::new(log_path.to_str().unwrap_or("/tmp/rektal_gui.log"))));
    Logger::global().add_sink(Box::new(TerminalSink { cli_prompt: None }));


    // Icon zur Compile-Zeit einbetten → kein Dateizugriff zur Laufzeit nötig
    let icon = {
        let image = img_crate::load_from_memory(include_bytes!("../assets/rektal-logo-without-title-32px.png"))
            .expect("Icon konnte nicht geladen werden")
            .into_rgba8();
        let width = image.width();
        let height = image.height();
        egui::viewport::IconData {
            rgba: image.into_raw(),
            width,
            height,
        }
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 768.0])
            .with_title("R.E.K.T.A.L.")
            .with_app_id("rektal")
            .with_icon(std::sync::Arc::new(icon)),
        ..Default::default()
    };
    r_log!(Info, "eframe initialized");

    eframe::run_native(
        "Docking App",
        options,
        Box::new(|cc| Ok(Box::new(MyApp::new(cc.egui_ctx.clone())))),
    )
}

/// The main application state.
/// Holds the docking tree, settings flags, user credentials, network states, and channels.
pub struct MyApp {
    /// The docking state tree holding all tabs.
    tree: DockState<Tab>,
    /// Whether the session settings window is visible.
    show_session_settings: bool,
    /// Whether the connection settings window is visible.
    show_connection_settings: bool,
    /// The IP address and port of the target server.
    server_address: String,
    /// The currently logged-in username.
    username: String,
    /// The ID assigned to the next created tab.
    next_tab_id: u32,
    /// Receiver channel for incoming DMX Universe data.
    dmx_receiver: Receiver<(u8, [u8; MAX_CHANNEL])>,
    /// Receiver channel for incoming TCP messages from the server.
    tcp_listen_receiver: Option<Receiver<TcpServerMessage>>,
    /// Sender channel for emitting UI events to the controller.
    ui_event_sender: Sender<UiEvent>,
    /// Receiver channel for handling UI events in the controller loop.
    ui_event_receiver: Receiver<UiEvent>,
    /// Sender channel for sending TCP messages to the server.
    tcp_write_sender: Option<Sender<TcpClientMessage>>,
    /// The current state of the TCP network connection.
    connection_state: ConnectionState,
    /// The currently active role of the user.
    role: UserRole,
    /// The password used during the login request (cleared from memory after use).
    password: String,
    /// Draft variable for the username, used in the settings window before applying.
    draft_username: String,
    /// Draft variable for the role, used in the settings window before applying.
    draft_role: UserRole,
}

impl MyApp {
    /// Creates a new instance of the application with default settings.
    /// Also initializes the UDP listener for DMX data and sets up the primary communication channels.
    fn new(ctx: egui::Context) -> Self {
        let (dmx_sender, dmx_receiver) = mpsc::channel();

        let (ui_event_sender, ui_event_receiver) = mpsc::channel();

        *UI_EVENT_SENDER.write().unwrap() = Some(ui_event_sender.clone());

        if let Err(e) = udp_client::start_udp_listener(None, dmx_sender, ctx) {
            r_log!(Error, "Failed to start UDP listener: {}", e);
        }

        let initial_terminal_panel = panels::terminal::TerminalPanel::new(0);

        Self {
            tree: DockState::new(vec![Tab::Terminal(initial_terminal_panel)]),
            show_session_settings: false,
            show_connection_settings: false,
            server_address: "127.0.0.1".to_string(),
            username: "Default User".to_string(),
            next_tab_id: 1,
            dmx_receiver,
            tcp_listen_receiver: None,
            ui_event_sender,
            ui_event_receiver,
            tcp_write_sender: None,
            connection_state: ConnectionState::Disconnected,
            role: UserRole::Programmer,
            password: String::new(),
            draft_username: String::new(),
            draft_role: UserRole::Programmer,
        }
    }

    /// Draws the top menu bar, containing the connection and session controls.
    fn draw_top_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("Connection", |ui| {
                    if ui.button("Connection Settings").clicked() {
                        self.show_connection_settings = true;
                        ui.close_menu();
                    }
                    if ui
                        .add_enabled(
                            matches!(self.connection_state, Connected { .. }),
                            egui::Button::new("Session Settings"),
                        )
                        .clicked()
                    {
                        self.show_session_settings = true;
                        self.draft_username = self.username.clone();
                        self.draft_role = self.role.clone();
                        self.password.clear(); // just to be safe
                        ui.close_menu();
                    }
                    if ui
                        .add_enabled(
                            matches!(
                                self.connection_state,
                                Connected {
                                    session_state: SessionState::LoggedIn
                                }
                            ),
                            egui::Button::new("Logout"),
                        )
                        .clicked()
                    {
                        let event = UiEvent::LogoutRequest;
                        if let Err(e) = self.ui_event_sender.send(event) {
                            r_log!(Error, "Failed to send UiEvent: {}", e);
                        }
                    }
                    if ui
                        .add_enabled(
                            !matches!(self.connection_state, ConnectionState::Disconnected),
                            egui::Button::new("Disconnect"),
                        )
                        .clicked()
                    {
                        let event = UiEvent::DisconnectRequest;
                        if let Err(e) = self.ui_event_sender.send(event) {
                            r_log!(Error, "Failed to send UiEvent: {}", e);
                        }
                    }
                });

                ui.menu_button("Window", |ui| {
                    if ui.button("Universe").clicked() {
                        let event = UiEvent::SubscribeRequest {topic: DMXConfiguration};
                        if let Err(e) = self.ui_event_sender.send(event) {
                            r_log!(Error, "Failed to send UiEvent: {}", e);
                        }

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
    }

    /// Draws the central docking panel, managing all active tabs (Terminals, Universes, etc.).
    fn draw_central_panel(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut tab_viewer = MyTabViewer {};
            DockArea::new(&mut self.tree)
                .style(egui_dock::Style::from_egui(ui.style().as_ref()))
                .show_inside(ui, &mut tab_viewer);
        });
    }

    /// Renders the Connection Settings pop-up window.
    /// Handles spawning the TCP client thread and establishing connections.
    fn draw_connection_settings(&mut self, ctx: &egui::Context) {
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
                            if ui.text_edit_singleline(&mut self.server_address).changed() {}
                        });
                    ui.add_space(10.0);

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                        if ui.button("Connect").clicked() {
                            let (tcp_write_sender, tcp_write_receiver) = mpsc::channel();
                            let (tcp_listen_sender, tcp_listen_receiver) = mpsc::channel();
                            self.tcp_write_sender = Some(tcp_write_sender);
                            self.tcp_listen_receiver = Some(tcp_listen_receiver);

                            let server_address_clone = self.server_address.clone();
                            std::thread::spawn(move || {
                                let mut tcp_client = TcpClient::new(
                                    format!("{}:6767", server_address_clone).parse().unwrap(),
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
    }

    /// Renders the Session Settings pop-up window.
    /// Handles inputting credentials and initiating the login sequence.
    fn draw_session_settings(&mut self, ctx: &egui::Context) {
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
                            ui.text_edit_singleline(&mut self.draft_username);
                            ui.end_row();

                            ui.label("Role");
                            egui::ComboBox::from_id_source("role_combo")
                                .selected_text(match self.draft_role {
                                    UserRole::Programmer => "Programmer",
                                    UserRole::BlindProgrammer => "Programmer Blind",
                                    UserRole::Showrunner => "Showrunner",
                                })
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut self.draft_role,
                                        UserRole::Programmer,
                                        "Programmer",
                                    );
                                    ui.selectable_value(
                                        &mut self.draft_role,
                                        UserRole::BlindProgrammer,
                                        "Programmer Blind",
                                    );
                                    ui.selectable_value(
                                        &mut self.draft_role,
                                        UserRole::Showrunner,
                                        "Showrunner",
                                    );
                                });
                            ui.end_row();

                            ui.label("Password");
                            let password_edit =
                                egui::TextEdit::singleline(&mut self.password).password(true);
                            ui.add(password_edit);
                            ui.end_row();
                        });
                    ui.add_space(10.0);

                    if let Connected {
                        session_state: SessionState::LoginFailed(ref reason),
                    } = self.connection_state
                    {
                        ui.label(
                            egui::RichText::new(format!("Login fehlgeschlagen: {}", reason))
                                .color(egui::Color32::RED),
                        );
                        ui.add_space(10.0);
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                        if ui.button("Login").clicked() {
                            self.username = self.draft_username.clone();
                            self.role = self.draft_role.clone();

                            let event = UiEvent::LoginRequest {
                                user_name: self.username.clone(),
                                password: self.password.clone(),
                                user_role: self.role.clone(),
                            };
                            if let Err(e) = self.ui_event_sender.send(event) {
                                r_log!(Error, "Failed to send UiEvent: {}", e);
                            } else {
                                request_close = true;
                            }
                            self.password.clear();
                        }
                        if ui.button("Close").clicked() {
                            request_close = true;
                        }
                    });
                });
        }
        if request_close {
            is_session_open = false;
        }
        self.show_session_settings = is_session_open;
    }

    /// Draws the bottom status bar, displaying connection state, user role, and software version.
    fn draw_bottom_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("bottom_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let status_color = match &self.connection_state {
                    ConnectionState::Disconnected | ConnectionState::Error => egui::Color32::RED,
                    ConnectionState::ConnectionPending => egui::Color32::YELLOW,
                    Connected { session_state } => match session_state {
                        SessionState::LoginFailed(_) => egui::Color32::YELLOW,
                        SessionState::LoggedIn => egui::Color32::GREEN,
                        SessionState::LoginPending | SessionState::LoggedOut => {
                            egui::Color32::YELLOW
                        }
                    },
                };
                let (rect, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
                ui.painter().circle_filled(rect.center(), 4.0, status_color);
                if let Connected {
                    session_state: SessionState::LoggedIn,
                } = self.connection_state
                {
                    ui.label(format!("{} | {}", self.username, self.role.to_string()));
                } else {
                    ui.label("Logged out");
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!(
                        "R.E.K.T.A.L. Version: {}",
                        env!("CARGO_PKG_VERSION")
                    ));
                });
            });
        });
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        controller::handle_dmx_data(&self.dmx_receiver, &mut self.tree);
        controller::handle_incoming_network_data(
            &mut self.tcp_listen_receiver,
            &mut self.tree,
        );
        controller::handle_events(
            &self.ui_event_receiver,
            &mut self.tcp_write_sender,
            &mut self.connection_state,
            &mut self.tree,
        );

        self.draw_top_bar(ctx);
        self.draw_bottom_bar(ctx);
        self.draw_central_panel(ctx);
        self.draw_connection_settings(ctx);
        self.draw_session_settings(ctx);
    }
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        r_log!(Info, "Shutting down..");

        if let Some(sender) = self.tcp_write_sender.take() {
            let _ = sender.send(TcpClientMessage::Logout);
        }

        send_ui_event(UiEvent::DisconnectRequest);
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

    fn id(&mut self, tab: &mut Self::Tab) -> egui::Id {
        egui::Id::new(tab.unique_id())
    }
}
