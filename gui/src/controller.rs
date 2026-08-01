use crate::network::connection_state::{ConnectionState, SessionState};
use crate::panels::Tab;
use common::networking::messages::{TcpClientMessage, TcpServerMessage};
use egui_dock::DockState;
use std::sync::mpsc::{Receiver, Sender};
use crate::panels::terminal::TextFragment;
use eframe::egui::Color32;
use common::logging::LogLevel;

pub(crate) fn handle_incoming_network_data(tcp_receiver: &mut Option<Receiver<TcpServerMessage>>, tree: &mut DockState<Tab>, session_state: &mut SessionState) {
    if let Some(tcp_receiver) = tcp_receiver{
        while let Ok(msg) = tcp_receiver.try_recv() {
            match msg {
                TcpServerMessage::CommandOutput { answer, response_id } => {
                    for (_, tab) in tree.iter_all_tabs_mut() {
                        if let Tab::Terminal(panel) = tab {
                            if panel.tab_id == response_id {
                                let (log_level, ref answer_string) = answer;
                                panel.add_fragments(vec![TextFragment {
                                    text: format!("[{}]: {}",log_level, answer_string),
                                    color: log_level_to_color32(log_level),
                                }]);

                            }
                        }
                    }
                }
                TcpServerMessage::LoginOk {token } => {
                    println!("\x1b[32m[System] Login Successful! Token: {}\x1b[0m", token);
                    *session_state = SessionState::LoggedIn;
                }
                TcpServerMessage::LoginFailed { reason } => {
                    println!("\x1b[91m[System] Login Failed: {}\x1b[0m", reason);
                    *session_state = SessionState::LoginFailed(reason);
                }
                _ => {}
            }
        }
    }
}

pub fn log_level_to_color32(level: LogLevel) -> Color32 {
    match level {
        LogLevel::SuccessEvent => Color32::GREEN,
        LogLevel::Info => Color32::BLUE,
        LogLevel::Warning => Color32::YELLOW,
        LogLevel::Error => Color32::RED,
        LogLevel::UserError => Color32::LIGHT_RED,
        LogLevel::UserSuccess => Color32::LIGHT_GREEN,
    }
}

pub enum UiEvent {
    SendTerminalCommand { id: u32, command: String },
    LoginRequest {
        password: String,
        user_name: String,
        user_role: common::networking::messages::UserRole,
    },
    SetConnectionState { state: ConnectionState },
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
                        eprintln!("Failed to send ExecuteCommand: {}", e);
                    }
                } else {
                    eprintln!("Failed to send ExecuteCommand: tcp sender doesn't exist");
                }
            }
            UiEvent::LoginRequest { password, user_name, user_role } => {
                let msg = TcpClientMessage::Login {
                    password,
                    user_name,
                    user_role,
                };
                if let Some(tcp_sender) = tcp_sender {
                    if let Err(e) = tcp_sender.send(msg) {
                        eprintln!("Failed to send LoginRequest: {}", e);
                    } else {
                        *session_state = SessionState::LoginPending;
                    }
                } else {
                    eprintln!("Failed to send ExecuteCommand: tcp sender doesn't exist");
                }
            }
            UiEvent::SetConnectionState {
                state,
            } => {
                *connection_state = state;
            }
            UiEvent::LogoutRequest => {
                let msg = TcpClientMessage::Logout;
                if let Some(tcp_sender) = tcp_sender {
                    if let Err(e) = tcp_sender.send(msg) {
                        eprintln!("Failed to send LogoutRequest: {}", e);
                    } else {
                        *session_state = SessionState::LoggedOut;
                    }
                } else {
                    eprintln!("Failed to send ExecuteCommand: tcp sender doesn't exist");
                }
            }
        }
    }
}
