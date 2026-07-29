use crate::network::connection_state::ConnectionState;
use crate::panels::Tab;
use common::networking::messages::{TcpClientMessage, TcpServerMessage};
use egui_dock::DockState;
use std::sync::mpsc::{Receiver, Sender};
use crate::panels::terminal::TextFragment;
use eframe::egui::Color32;

pub(crate) fn handle_incoming_network_data(tcp_receiver: &mut Option<Receiver<TcpServerMessage>>, tree: &mut DockState<Tab>, connection_state: &mut ConnectionState) {
    if let Some(tcp_receiver) = tcp_receiver{
        while let Ok(msg) = tcp_receiver.try_recv() {
            match msg {
                TcpServerMessage::CommandOutput { answer, terminal_id } => {
                    for (_, tab) in tree.iter_all_tabs_mut() {
                        if let Tab::Terminal(panel) = tab {
                            if panel.tab_id == terminal_id {
                                match answer {
                                    Ok(ref response) => {
                                        panel.add_fragments(vec![TextFragment {
                                            text: format!("[Server]: {}", response),
                                            color: Color32::LIGHT_GRAY,
                                        }]);
                                    }
                                    Err(ref e) => {
                                        panel.add_fragments(vec![TextFragment {
                                            text: format!("[Server Error]: {}", e),
                                            color: Color32::RED,
                                        }]);
                                    }
                                }
                            }
                        }
                    }
                }
                TcpServerMessage::LoginOk {token } => {
                    println!("\x1b[32m[System] Login Successful! Token: {}\x1b[0m", token);
                    *connection_state = ConnectionState::LoggedIn;
                }
                TcpServerMessage::LoginFailed { reason } => {
                    println!("\x1b[91m[System] Login Failed: {}\x1b[0m", reason);
                    *connection_state = ConnectionState::LoginFailed(reason);
                }
                _ => {}
            }
        }
    }
}

pub enum UiEvent {
    SendTerminalCommand { id: u32, command: String },
    LoginRequest {
        password: String,
        user_name: String,
        user_role: common::networking::messages::UserRole,
    },
    SetConnectionState { state: ConnectionState }
}

pub(crate) fn handle_events(
    ui_receiver: &Receiver<UiEvent>,
    tcp_sender: &Option<Sender<TcpClientMessage>>,
    connection_state: &mut ConnectionState,
) {
    while let Ok(event) = ui_receiver.try_recv() {
        match event {
            UiEvent::SendTerminalCommand { id, command } => {
                let msg = TcpClientMessage::ExecuteCommand {
                    terminal_id: id,
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
                        *connection_state = ConnectionState::LoginPending;
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
        }
    }
}
