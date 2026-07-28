use crate::panels::Tab;
use common::networking::messages::{TcpClientMessage, TcpServerMessage};
use egui_dock::DockState;
use std::sync::mpsc::{Receiver, Sender};
use crate::panels::terminal::TextFragment;
use eframe::egui::Color32;

pub(crate) fn handle_incoming_network_data(tcp_receiver: &mut Receiver<TcpServerMessage>, tree: &mut DockState<Tab>) {
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
            _ => {}
        }
    }
}

pub enum UiEvent {
    SendTerminalCommand { id: u32, command: String },
    LoginRequest,
}

pub(crate) fn handle_outgoing_events(
    ui_receiver: &Receiver<UiEvent>,
    tcp_sender: &Sender<TcpClientMessage>,
) {
    while let Ok(event) = ui_receiver.try_recv() {
        match event {
            UiEvent::SendTerminalCommand { id, command } => {
                let msg = TcpClientMessage::ExecuteCommand {
                    terminal_id: id,
                    command,
                };
                if let Err(e) = tcp_sender.send(msg) {
                    eprintln!("Failed to send ExecuteCommand: {}", e);
                }
            }
            UiEvent::LoginRequest => {
                let msg = TcpClientMessage::Login {
                    password: "".to_string(),
                    user_name: "loetgott".to_string(),
                    user_role: common::networking::messages::UserRole::Programmer,
                };
                if let Err(e) = tcp_sender.send(msg) {
                    eprintln!("Failed to send LoginRequest: {}", e);
                }
            }
        }
    }
}
