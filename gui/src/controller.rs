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

fn send_ui_event(event: UiEvent) {
    if let Ok(guard) = crate::UI_EVENT_SENDER.read() {
        if let Some(sender) = guard.as_ref() {
            let _ = sender.send(event);
        }
    }
}

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
                    send_ui_event(UiEvent::SetSessionState {
                        state: SessionState::LoggedIn,
                    });
                }
                TcpServerMessage::LoginFailed { reason } => {
                    r_log!(UserError, "Login Failed: {}", reason);
                    send_ui_event(UiEvent::SetSessionState {
                        state: SessionState::LoginFailed(reason),
                    });
                }
                TcpServerMessage::LogoutOk => {
                    r_log!(UserSuccess, "Logout Successful");
                    send_ui_event(UiEvent::SetSessionState {
                        state: SessionState::LoggedOut,
                    });
                }
                TcpServerMessage::TopicUpdate { data } => {
                    r_log!(Info, "{:?}", data.get_topic().to_string());
                    match data {
                        TopicPayload::DMXConfiguration(dmx_config) => {
                            for (_, tab) in tree.iter_all_tabs_mut() {
                                if let Tab::Universe(panel) = tab {
                                    panel.device_configuration = Some(dmx_config.clone());
                                }
                            }
                        }
                    }
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
    SetSessionState {
        state: SessionState,
    },
    LogoutRequest,
    SubscribeRequest {
        topic: SubscribeTopic,
    },
}

pub(crate) fn handle_events(
    ui_receiver: &Receiver<UiEvent>,
    tcp_sender: &Option<Sender<TcpClientMessage>>,
    connection_state: &mut ConnectionState,
    session_state: &mut SessionState,
    tree: &mut DockState<Tab>,
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
                if state == ConnectionState::Error {
                    *session_state = SessionState::LoggedOut;
                    for (_, tab) in tree.iter_all_tabs_mut() {
                        if let Tab::Terminal(panel) = tab {
                            panel.is_active = false;
                            panel.add_fragments(vec![TextFragment {
                                text: "[ERROR] There was an Error with the connection!".to_string(),
                                color: Color32::RED,
                            }]);
                        }
                    }
                }

                if state == ConnectionState::Disconnected {
                    *session_state = SessionState::LoggedOut;
                    for (_, tab) in tree.iter_all_tabs_mut() {
                        if let Tab::Terminal(panel) = tab {
                            panel.is_active = false;
                            panel.add_fragments(vec![TextFragment {
                                text: "[INFO] Kernel disconnected!".to_string(),
                                color: Color32::LIGHT_BLUE,
                            }]);
                        }
                    }
                }
                *connection_state = state;
            }
            UiEvent::SetSessionState { state } => {
                match &state {
                    SessionState::LoggedIn => {
                        for (_, tab) in tree.iter_all_tabs_mut() {
                            if let Tab::Terminal(panel) = tab {
                                panel.is_active = true;
                                panel.add_fragments(vec![TextFragment {
                                    text: "[INFO] Connection established, terminal ready!".to_string(),
                                    color: Color32::LIGHT_GREEN,
                                }]);
                            }
                        }
                    }
                    SessionState::LoggedOut | SessionState::LoginFailed(_) => {
                        for (_, tab) in tree.iter_all_tabs_mut() {
                            if let Tab::Terminal(panel) = tab {
                                panel.is_active = false;
                                panel.add_fragments(vec![TextFragment {
                                    text: "[INFO] User logged out! Log in to use Terminal".to_string(),
                                    color: Color32::YELLOW,
                                }]);
                            }
                        }
                    }
                    SessionState::LoginPending => {}
                }
                *session_state = state;
            }
            UiEvent::LogoutRequest => {
                let msg = TcpClientMessage::Logout;
                if let Some(tcp_sender) = tcp_sender {
                    if let Err(e) = tcp_sender.send(msg) {
                        r_log!(Error, "Failed to send logout request: {}", e);
                    } else {
                        for (_, tab) in tree.iter_all_tabs_mut() {
                            if let Tab::Terminal( panel) = tab {
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
