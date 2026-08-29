//! # Controller Module
//!
//! The `controller` module acts as the central event handler and coordinator
//! for the R.E.K.T.A.L. GUI application.
//!
//! It is responsible for:
//! - Draining incoming DMX universe stream data and updating universe UI panels.
//! - Dispatching global UI events ([`UiEvent`]) via channels.
//! - Processing network messages ([`TcpServerMessage`]) received from the kernel server.
//! - Handling UI action events and updating central application states ([`ConnectionState`], [`SessionState`]).

use crate::network::connection_state::SessionState::{LoggedIn, LoggedOut, LoginFailed};
use crate::network::connection_state::{ConnectionState, SessionState};
use crate::network::udp_client::MAX_CHANNEL;
use crate::panels::terminal::TextFragment;
use crate::panels::Tab;
use common::logging::LogLevel;
use common::logging::LogLevel::*;
use common::networking::messages::{TcpClientMessage, TcpServerMessage};
use common::networking::subscription_objects::{SubscribeTopic, TopicPayload};
use common::r_log;
use eframe::egui::Color32;
use egui_dock::DockState;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::sync::mpsc::{Receiver, Sender};

/// Enum describing global GUI events dispatched across the application.
///
/// These events represent user interactions or system actions that require
/// coordination between UI panels, the central state, or network threads.
pub enum UiEvent {
    /// Sends a command entered in a terminal panel to the kernel server (or processes built-in commands).
    SendTerminalCommand {
        /// The unique ID of the terminal tab originating the command.
        id: u32,
        /// The command string entered by the user.
        command: String,
    },
    /// Initiates a login request to the kernel server with user credentials.
    LoginRequest {
        /// The plain text password entered by the user.
        password: String,
        /// The username for authentication.
        user_name: String,
        /// The requested user role (e.g., Programmer, Showrunner).
        user_role: common::networking::messages::UserRole,
    },
    /// Updates the central connection and session states of the GUI application.
    SetConnectionState {
        /// The target connection state to apply.
        state: ConnectionState,
    },
    /// Requests a user session logout from the kernel server.
    LogoutRequest,
    /// Forcefully disconnects the TCP network connection.
    DisconnectRequest,
    /// Subscribes to a data topic (e.g. DMX configuration updates) from the server.
    SubscribeRequest {
        /// The subscription topic requested.
        topic: SubscribeTopic,
    },
}

impl Display for UiEvent {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            UiEvent::SendTerminalCommand { id, command } => {
                write!(f, "SendTerminalCommand, id: {}, command: {}", id, command)
            }
            UiEvent::LoginRequest {
                user_name,
                password,
                user_role,
            } => write!(
                f,
                "LoginRequest, username: {},password: {}, role: {}",
                user_name, password, user_role
            ),
            UiEvent::SetConnectionState { state } => write!(f, "SetConnectionState: {}", state),
            UiEvent::LogoutRequest => write!(f, "LogoutRequest"),
            UiEvent::DisconnectRequest => write!(f, "DisconnectRequest"),
            UiEvent::SubscribeRequest { topic } => write!(f, "SubscribeRequest, topic: {}", topic),
        }
    }
}

/// Drains incoming DMX universe frame data from the UDP receiver channel
/// and updates the matching Universe panels in the docking tree.
///
/// # Arguments
/// * `dmx_receiver` - Receiver channel producing `(universe_id, dmx_data)` tuples.
/// * `tree` - Mutable reference to the UI docking tree containing active tabs.
pub(crate) fn handle_dmx_data(
    dmx_receiver: &Receiver<(u8, [u8; MAX_CHANNEL])>,
    tree: &mut DockState<Tab>,
) {
    while let Ok((universe_id, dmx_data)) = dmx_receiver.try_recv() {
        for (_, tab) in tree.iter_all_tabs_mut() {
            if let Tab::Universe(panel) = tab {
                if panel.selected_universe - 1 == universe_id {
                    panel.dmx_data.copy_from_slice(&dmx_data);
                }
            }
        }
    }
}

/// Thread-safe helper to send a [`UiEvent`] into the global [`UI_EVENT_SENDER`] channel.
///
/// # Arguments
/// * `event` - The [`UiEvent`] to send.
pub fn send_ui_event(event: UiEvent) {
    if let Ok(guard) = crate::UI_EVENT_SENDER.read() {
        if let Some(sender) = guard.as_ref() {
            let _ = sender.send(event);
        }
    }
}

/// Processes a command entered into a terminal tab.
///
/// Handles built-in local terminal commands (like `"logout"`) directly or packages
/// generic commands into a [`TcpClientMessage::ExecuteCommand`] to send to the server.
///
/// # Arguments
/// * `id` - The unique ID of the terminal tab.
/// * `command` - The command string entered by the user.
/// * `tcp_sender` - Optional sender channel to transmit messages to the TCP network thread.
/// * `tree` - Mutable reference to the UI docking tree to update terminal outputs.
fn process_terminal_command(
    id: u32,
    command: String,
    tcp_sender: &Option<Sender<TcpClientMessage>>,
    tree: &mut DockState<Tab>,
) {
    match command.as_str() {
        "logout" => {
            send_ui_event(UiEvent::LogoutRequest);
            for (_, tab) in tree.iter_all_tabs_mut() {
                if let Tab::Terminal(panel) = tab {
                    panel.add_fragments(vec![TextFragment {
                        text: "[SUCCESS] Logout successful, Terminal inactive!".to_string(),
                        color: log_level_to_color32(SuccessEvent),
                    }]);
                };
            }
        }
        _ => {
            let msg = TcpClientMessage::ExecuteCommand {
                response_id: id,
                command,
            };
            if let Some(tcp_sender) = tcp_sender {
                if let Err(e) = tcp_sender.send(msg) {
                    r_log!(Error, "Failed to send execute command: {}", e);
                }
            } else {
                r_log!(
                    Error,
                    "Failed to send execute command: tcp sender doesn't exist"
                );
            }
        }
    }
}

/// Maps a common [`LogLevel`] enum variant to a corresponding `egui` [`Color32`] for UI rendering.
///
/// # Arguments
/// * `level` - The log level to convert.
///
/// # Returns
/// An `egui::Color32` representing the log level.
pub fn log_level_to_color32(level: LogLevel) -> Color32 {
    match level {
        SuccessEvent => Color32::GREEN,
        Info => Color32::BLUE,
        Warning => Color32::YELLOW,
        Error => Color32::RED,
        UserError => Color32::GOLD,
        UserSuccess => Color32::LIGHT_GREEN,
    }
}

/// Drains incoming network messages ([`TcpServerMessage`]) from the TCP receiver channel
/// and updates UI tabs or triggers connection state changes.
///
/// # Arguments
/// * `tcp_receiver` - Mutable reference to the optional TCP network message receiver channel.
/// * `tree` - Mutable reference to the UI docking tree.
pub(crate) fn handle_incoming_network_data(
    tcp_receiver: &mut Option<Receiver<TcpServerMessage>>,
    tree: &mut DockState<Tab>,
) {
    if let Some(tcp_receiver) = tcp_receiver {
        while let Ok(msg) = tcp_receiver.try_recv() {
            match msg {
                TcpServerMessage::CommandOutput {
                    answer,
                    response_id,
                } => {
                    for (_, tab) in tree.iter_all_tabs_mut() {
                        if let Tab::Terminal(panel) = tab {
                            if panel.tab_id == response_id {
                                let (log_level, ref answer_string) = answer;
                                panel.add_fragments(vec![TextFragment {
                                    text: format!("[{}]: {}", log_level, answer_string),
                                    color: log_level_to_color32(log_level),
                                }]);
                            }
                        }
                    }
                }
                TcpServerMessage::LoginOk { token } => {
                    r_log!(UserSuccess, "Login Successful! Token: {}", token);
                    send_ui_event(UiEvent::SetConnectionState {
                        state: ConnectionState::Connected {
                            session_state: LoggedIn,
                        },
                    });
                }
                TcpServerMessage::LoginFailed { reason } => {
                    r_log!(UserError, "Login Failed: {}", reason);
                    send_ui_event(UiEvent::SetConnectionState {
                        state: ConnectionState::Connected {
                            session_state: LoginFailed(reason),
                        },
                    });
                }
                TcpServerMessage::LogoutOk => {
                    r_log!(UserSuccess, "Logout Successful");
                    send_ui_event(UiEvent::SetConnectionState {
                        state: ConnectionState::Connected {
                            session_state: LoggedOut,
                        },
                    });
                }
                TcpServerMessage::TopicUpdate { data } => match data {
                    TopicPayload::DMXConfiguration(dmx_config) => {
                        for (_, tab) in tree.iter_all_tabs_mut() {
                            if let Tab::Universe(panel) = tab {
                                panel.device_configuration = Some(dmx_config.clone());
                            }
                        }
                    }
                },
                _ => {}
            }
        }
    }
}

/// Drains and processes all queued [`UiEvent`] items sent from UI interactions or network handlers.
///
/// Manages connection state transitions, sends outgoing TCP messages, and activates/deactivates
/// UI panels in the docking tree accordingly.
///
/// # Arguments
/// * `ui_receiver` - Receiver channel for queued [`UiEvent`]s.
/// * `tcp_sender` - Mutable reference to the optional TCP client sender channel.
/// * `connection_state` - Mutable reference to the application's central [`ConnectionState`].
/// * `tree` - Mutable reference to the UI docking tree.
pub(crate) fn handle_events(
    ui_receiver: &Receiver<UiEvent>,
    tcp_sender: &mut Option<Sender<TcpClientMessage>>,
    connection_state: &mut ConnectionState,
    tree: &mut DockState<Tab>,
) {
    while let Ok(event) = ui_receiver.try_recv() {
        r_log!(Info, "UIEvent: {}" ,event);
        match event {
            UiEvent::SendTerminalCommand { id, command } => {
                process_terminal_command(id, command, tcp_sender, tree);
            }
            UiEvent::LoginRequest {
                password,
                user_name,
                user_role,
            } => {
                let msg = TcpClientMessage::Login {
                    password,
                    user_name,
                    user_role,
                };
                if let Some(tcp_sender) = tcp_sender {
                    if let Err(e) = tcp_sender.send(msg) {
                        r_log!(Error, "Failed to send login request: {}", e);
                    } else {
                        *connection_state = ConnectionState::Connected {
                            session_state: SessionState::LoginPending,
                        };
                    }
                } else {
                    r_log!(
                        Error,
                        "Failed to send execute command: tcp sender doesn't exist"
                    );
                }
            }
            UiEvent::SetConnectionState { state } => {
                r_log!(Info, "New ConnectionState: {}", state);
                for (_, tab) in tree.iter_all_tabs_mut() {
                    if state
                        == (ConnectionState::Connected {
                            session_state: LoggedIn,
                        })
                    {
                        tab.on_connect();
                    }
                    if let Tab::Terminal(panel) = tab {
                        panel.is_active = state
                            == (ConnectionState::Connected {
                                session_state: LoggedIn,
                            });
                        if state
                            == (ConnectionState::Connected {
                                session_state: LoggedIn,
                            })
                        {
                            panel.add_fragments(vec![TextFragment {
                                text: "[INFO] Logins successful, Terminal ready!".to_string(),
                                color: Color32::GREEN,
                            }]);
                        }
                    }
                }
                *connection_state = state;
            }
            UiEvent::LogoutRequest => {
                let msg = TcpClientMessage::Logout;
                if let Some(tcp_sender) = tcp_sender {
                    if let Err(e) = tcp_sender.send(msg) {
                        r_log!(Error, "Failed to send logout request: {}", e);
                    } else {
                        for (_, tab) in tree.iter_all_tabs_mut() {
                            if let Tab::Terminal(panel) = tab {
                                panel.is_active = false;
                            }
                        }
                    }
                } else {
                    r_log!(
                        Error,
                        "Failed to send logout request: tcp sender doesn't exist"
                    );
                }
            }
            UiEvent::DisconnectRequest => {
                r_log!(Info, "Closing connection requested via UI");
                *tcp_sender = None;
                *connection_state = ConnectionState::Disconnected;
            }
            UiEvent::SubscribeRequest { topic } => {
                let msg = TcpClientMessage::Subscribe {
                    topic: topic.clone(),
                    update_mode: common::networking::subscription_objects::UpdateMode::OnChange,
                };
                if let Some(tcp_sender) = tcp_sender {
                    if let Err(e) = tcp_sender.send(msg) {
                        r_log!(Error, "Failed to send subscribe request: {}", e);
                    }
                }
            }
        }
    }
}
