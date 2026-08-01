use crate::network::connection_state::{ConnectionState, SessionState};
use crate::network::udp_client::MAX_CHANNEL;
use crate::panels::terminal::TextFragment;
use crate::panels::Tab;
use common::logging::LogLevel;
use common::logging::LogLevel::*;
use common::networking::messages::{TcpClientMessage, TcpServerMessage};
use common::r_log;
use eframe::egui::Color32;
use egui_dock::DockState;
use std::sync::mpsc::{Receiver, Sender};

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

pub(crate) fn handle_incoming_network_data(
    tcp_receiver: &mut Option<Receiver<TcpServerMessage>>,
    tree: &mut DockState<Tab>,
    session_state: &mut SessionState,
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
                    *session_state = SessionState::LoggedIn;
                }
                TcpServerMessage::LoginFailed { reason } => {
                    r_log!(UserError, "Login Failed: {}", reason);
                    *session_state = SessionState::LoginFailed(reason);
                }
                _ => {}
            }
        }
    }
}

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

pub enum UiEvent {
    SendTerminalCommand {
        id: u32,
        command: String,
    },
    LoginRequest {
        password: String,
        user_name: String,
        user_role: common::networking::messages::UserRole,
    },
    SetConnectionState {
        state: ConnectionState,
    },
    LogoutRequest,
}

pub(crate) fn handle_events(
    ui_receiver: &Receiver<UiEvent>,
    tcp_sender: &Option<Sender<TcpClientMessage>>,
    connection_state: &mut ConnectionState,
    session_state: &mut SessionState,
) {
    while let Ok(event) = ui_receiver.try_recv() {
        match event {
            UiEvent::SendTerminalCommand { id, command } => {
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
                        *session_state = SessionState::LoginPending;
                    }
                } else {
                    r_log!(
                        Error,
                        "Failed to send execute command: tcp sender doesn't exist"
                    );
                }
            }
            UiEvent::SetConnectionState { state } => {
                if state == ConnectionState::Disconnected || state == ConnectionState::Error {
                    *session_state = SessionState::LoggedOut;
                }
                *connection_state = state;
            }
            UiEvent::LogoutRequest => {
                let msg = TcpClientMessage::Logout;
                if let Some(tcp_sender) = tcp_sender {
                    if let Err(e) = tcp_sender.send(msg) {
                        r_log!(Error, "Failed to send logout request: {}", e);
                    } else {
                        *session_state = SessionState::LoggedOut;
                    }
                } else {
                    r_log!(
                        Error,
                        "Failed to send logout request: tcp sender doesn't exist"
                    );
                }
            }
        }
    }
}
