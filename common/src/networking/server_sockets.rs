use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::{thread};
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver,Sender};
use rand::{RngExt};
use crate::logging::LogLevel::*;
use crate::networking::connection_engine::{ClientSession, ConnectionID, SessionID, NEXT_CONNECTION_ID, SERVER_STATE};
use crate::networking::messages::{HandshakeRequest, HandshakeResponse, SubscribeTopic, TcpClientMessage, TcpServerMessage, UpdateMode};
use crate::networking::messages::TcpServerMessage::{CommandOutput, LogoutOk};

pub fn activate_socket<F>(port: u16, command_handler: F)
where F : Fn(String) -> Result<String, String> + Send + Sync + 'static + Clone {
    let address = format!("0.0.0.0:{}", port);
    let listener = match TcpListener::bind(&address) {
        Ok(l) => l,
        Err(e) => {
            r_log!(Error,"CRITICAL: Failed to bind TCP socket on address {}. Is another kernel instance already \
                    running? OS Error: {}",address, e);

            std::process::exit(1);
        }
    };

    r_log!(Info,"Listening on {} for Clients", listener.local_addr().unwrap());

    thread::spawn(move|| {
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let connection_id = NEXT_CONNECTION_ID.fetch_add(1, Ordering::Relaxed);
                    r_log!(SuccessEvent,"[Conn {}] New client connected: {}",
                        connection_id, stream.peer_addr().unwrap());

                    let handler_clone = command_handler.clone();

                    thread::spawn(move || {
                        handle_client(stream, handler_clone, connection_id);
                    });
                }
                Err(e) => {
                    r_log!(Error,"Got error trying to establish connection : {}", e);
                }
            }
        }
    });
}

fn handle_client<F>(mut stream: TcpStream, command_handler: F, connection_id: ConnectionID)
where
    F : Fn(String) -> Result<String, String> + Send + Sync + 'static + Clone
{

    match check_version_compatibility(&mut stream) {
        Err(HandshakeError::InvalidData(e)) => {
            r_log!(Error,"[Conn {}] Handshake Error: {}. Connection Closed.", connection_id, e);
            return;
        }

        Err(HandshakeError::VersionMismatch {client_version, server_version}) => {
            r_log!(Warning,"[Conn {}] Version Mismatch: Client: {}, Server: {}. Connection Closed",
                     connection_id, client_version, server_version);
            return;
        }

        Ok(()) => {
            r_log!(Info,"[Conn {}] Versions match. Handshake completed.", connection_id);
        }
    }

    let (tx_channel, rx_channel) = mpsc::channel::<TcpServerMessage>();
    let write_stream = stream.try_clone().unwrap();

    thread::spawn(move|| {
        write_thread(rx_channel, write_stream, connection_id);
    });


    read_thread(&mut stream, command_handler, connection_id, &tx_channel);

}

struct VersionMismatch {
    server_version: String,
    client_version: String,
}

enum HandshakeError {
    VersionMismatch {
        server_version: String,
        client_version: String,
    },
    InvalidData(String),
}

fn check_version_compatibility(stream: &mut TcpStream) -> Result<(), HandshakeError> {
    let mut buffer = [0; 1024];

    let bytes = match stream.read(&mut buffer) {
        Ok(0) => return Err(HandshakeError::InvalidData("Connection was terminated instantly".into())),
        Ok(b) => b,
        Err(e) => return Err(HandshakeError::InvalidData(format!("Read-Error: {}", e)))
    };

    match bincode::deserialize::<HandshakeRequest>(&buffer[..bytes]) {
        Ok(request) => {
            if request.magic_string != "REKTAL" {
                return Err(HandshakeError::InvalidData("Wrong magic string. Client is not an Rektal-Client".into()));
            }

            let server_hash = crate::networking::messages::get_protocol_version();
            let server_version = env!("CARGO_PKG_VERSION").to_string();

            if request.protocol_hash != server_hash {
                let response = HandshakeResponse::Mismatch {
                    server_version: server_version.clone(),
                };

                let _ = stream.write(&bincode::serialize(&response).unwrap());

                Err(HandshakeError::VersionMismatch {
                    server_version,
                    client_version: request.client_version
                })
            } else {
                let response = HandshakeResponse::Ok;
                let _ = stream.write(&bincode::serialize(&response).unwrap());

                Ok(())
            }

        }
        Err(_) => Err(HandshakeError::InvalidData("Couldnt deserialize handshake".into())),
    }

}

fn read_thread<F>(stream: &mut TcpStream, command_handler: F, connection_id: ConnectionID, tx_channel: &Sender<TcpServerMessage>)
where
    F: Fn(String) -> Result<String, String> + Send + Sync + 'static + Clone
{
    let mut buffer = [0; 1024];
    let mut token: Option<SessionID> = None;

    loop {
        let bytes = match stream.read(&mut buffer) {
            Ok(0) => {
                r_log!(SuccessEvent,"[Conn {}] Client disconnected successfully", connection_id);
                break;
            }
            Err(e) => {
                r_log!(Warning,"[Conn {}] Connection error : {}", connection_id, e);
                break;
            }
            Ok(bytes) => bytes
        };

        let msg = bincode::deserialize::<TcpClientMessage>(&buffer[..bytes]).unwrap();
        r_log!(Info,"[Conn {}] Received Enum: {:?}", connection_id, msg);

        token = update_login_status(&msg, token, &tx_channel, connection_id);

        let Some(_) = token else {
            if !matches!(msg, TcpClientMessage::Logout | TcpClientMessage::Relogin {..} | TcpClientMessage::Login {..}){
                r_log!(UserError,"[Conn {}] Message discarded: Client is not authorized.", connection_id);
                let _ = tx_channel.send(TcpServerMessage::Unauthenticated);
            }
            continue
        };

        match msg {
            TcpClientMessage::Login { .. } | TcpClientMessage::Relogin { .. } => {
                //Already got handled by update_login_status
            },
            TcpClientMessage::Logout => { unreachable!() },

            _ => {
                if let Some(response) = handle_messages(msg, command_handler.clone(), connection_id) {
                    if let Err(e) = tx_channel.send(response) {
                        r_log!(Error,"[Conn {}] Got error while sending response to channel: {}", connection_id, e);
                    }
                }
            }
        }
    }

    //Clean Up
    if let Some(token) = token {
        let mut state = SERVER_STATE.write().unwrap();
        if let Some(session) = state.get_mut(&token) {
            match session.active_connection.as_ref().map(|tuple| tuple.1) {
                Some(active_id) if active_id == connection_id => {
                    session.active_connection = None;
                    r_log!(Info,"[Conn {}] Cleanup complete. Session {} is now sleeping", connection_id, token);
                }

                Some(active_id) => {
                    r_log!(Info,"[Conn {}] Cleanup for Session {} aborted. Session already has new Connection {}",
                             connection_id, token, active_id);
                }

                None => {
                    r_log!(Info,"[Conn {}] Cleanup for Session {} aborted. Session is already sleeping.",
                             connection_id, token);
                }
            }
        };
    }
    r_log!(Info,"The Read-Thread  {} is dead! Long live the Read-Thread!", connection_id);
}

fn write_thread(rx_channel: Receiver<TcpServerMessage>, mut write_stream: TcpStream, connection_id: ConnectionID) {
    while let Ok(message) = rx_channel.recv() {
        match bincode::serialize(&message) {
            Ok(serialized_response) => {
                if let Err(e) = write_stream.write_all(&serialized_response) {
                    r_log!(Warning,"[Conn {}] Got error while sending to Client: {} Stopped write-Thread",
                             connection_id, e);
                    break;
                }
            }

            Err(e) => {
                r_log!(Error,"[Conn {}] Got error while serializing response: {}", connection_id, e);
                break;
            }
        }

        if let TcpServerMessage::Kicked { reason } = message {
            r_log!(Info,"[Conn {}] Write thread terminating because client was kicked: {}", connection_id, reason);

            break;
        }
    }

    //Thread the Ripper, we kill the read-thread with us
    let _ = write_stream.shutdown(Shutdown::Both);
    r_log!(Info,"The Write-Thread {} is dead! Long live the Write-Thread!", connection_id);
}

fn update_login_status(
    message: &TcpClientMessage, old_token: Option<SessionID>, tx_channel: &Sender<TcpServerMessage>,
    connection_id:ConnectionID
) -> Option<SessionID> {
    let (new_token, response) = match message {
        TcpClientMessage::Login {password, user_name, user_role }  => {
            if let Some(real_old_token) = old_token {
                r_log!(UserError,
                    "[Conn {}] User '{}' tried to login, but connection is already logged in with token {}. Ignored.",
                    connection_id, user_name, real_old_token);
            }

            if password == "" {//TODO Passwort irgendwo speichern und hier auslesen
                let mut state = SERVER_STATE.write().unwrap();

                let mut rng = rand::rng();
                let new_token = loop {
                    let token:SessionID = rng.random();

                    if !state.contains_key(&token) {
                        break token;
                    }

                };

                state.insert(new_token, ClientSession {
                    user_name: user_name.clone(),
                    user_role: *user_role,
                    active_connection: Some((tx_channel.clone(), connection_id)),
                });

                r_log!(SuccessEvent,"[Conn {}] Logged in with Session-Token {}", connection_id, new_token);

                (Some(new_token), Some(TcpServerMessage::LoginOk { token: new_token }))
            } else {
                r_log!(UserError,"[Conn {}] User {} wanted to login with wrong password",
                         connection_id, user_name);
                let _ = tx_channel.send(TcpServerMessage::LoginFailed {
                    reason: String::from("Wrong password")
                });

                (None, Some(TcpServerMessage::LoginFailed { reason: "Wrong password".into() }))
            }
        }

        TcpClientMessage::Relogin {user_id, clear_subscriptions} => {
            if let Some(old_token) = old_token {
                r_log!(UserError,"[Conn {}] User with ID {} wanted to relog with ID {}, but he was logged \
                in. Relogin ignored", connection_id, old_token, user_id);
                (Some(old_token), Some(TcpServerMessage::ReloginFailed { reason: "Already logged in".into()}))

            }else if let Some(session) = SERVER_STATE.write().unwrap().get_mut(&user_id) {

                if let Some((old_user_channel,old_connection_id)) = session.active_connection.take() {
                    let _ = old_user_channel.send(TcpServerMessage::Kicked {
                        reason: "Newer Connection relogged in with same token".into()
                    });
                    r_log!(Warning,"[Conn {}] User was kicked due to newer Connection with same token {}. \
                    Killed old Connection {}", connection_id, user_id, old_connection_id);
                }

                session.active_connection = Some((tx_channel.clone(), connection_id));

                if *clear_subscriptions {
                    //TODO Clear Subscriptions, wenn die Infrastruktur dafür da ist
                }

                r_log!(SuccessEvent,"[Conn {}] User with ID {} relogged in  successfully", connection_id, user_id);
                (Some(*user_id), Some(TcpServerMessage::ReloginOk { token: *user_id}))
            } else {
                r_log!(Warning,"[Conn {}] User wanted to relogin with ID {}, wich doesnt exist",
                         connection_id, user_id);
                (None, Some(TcpServerMessage::ReloginFailed { reason: "User doesnt exist".into() }))
            }
        }

        TcpClientMessage::Logout => {
            if let Some(old_token) = old_token {
                let mut state = SERVER_STATE.write().unwrap();

                state.remove(&old_token);
                r_log!(SuccessEvent,"[Conn {}] Client logged out successfully", connection_id);
            } else {
                //User wanted to log out, but wasn't logged in in the first place
                //No log message here, because everyone is happy
            }
            (None, Some(LogoutOk))
        }

        _ => (old_token, None)
    };

    if let Some(response) = response {
        if let Err(e) = tx_channel.send(response) {
            r_log!(Error,"[Conn {}] Fehler beim Senden der Auth-Antwort an den Channel: {}", connection_id, e);
        }
    }

    new_token
}

fn handle_messages<F>(msg: TcpClientMessage, command_handler: F, connection_id: ConnectionID) -> Option<TcpServerMessage>
where F : Fn(String) -> Result<String, String> + Send + Sync + 'static + Clone {
    match msg {
        TcpClientMessage::Login {..} | TcpClientMessage::Logout | TcpClientMessage::Relogin {..} => unreachable!(),


        TcpClientMessage::Subscribe {topic, update_mode} => {
            let topic_name = match topic {
                SubscribeTopic::FixturePositions => "Fixture Positions",
                SubscribeTopic::Universes => "Universes",
            };
            r_log!(Info,"[Conn {}] {}", connection_id, match update_mode {
                UpdateMode::OnChange => format!("Der Client will über Änderungen von {} erfahren!", topic_name),
                UpdateMode::Continuous => format!("Der Client will über {} auf dem laufenden gehalten werden", topic_name),
            }); //TODO Normally we always should respond to this
            None
        }

        TcpClientMessage::Unsubscribe { topic } => {
            r_log!(Info,"[Conn {}] Der Client will nichts von {} wissen.", connection_id, match topic {
                SubscribeTopic::FixturePositions => "Fixture Positions",
                SubscribeTopic::Universes => "Universes",
            });
            None
        }

        TcpClientMessage::ExecuteCommand{ command, terminal_id} => {
            let response = command_handler(command);
            match response.clone() {
                Ok(response_ok) => {
                    r_log!(UserSuccess,"[Conn {}] {}", connection_id, response_ok);
                }
                Err(response_error) => {
                    r_log!(UserError,"[Conn {}] {}", connection_id, response_error);
                }
            }
            Some(CommandOutput{
                answer: response,
                terminal_id
            })
        }

        TcpClientMessage::RequestEdit(_) => {
            r_log!(Info,"[Conn {}] Yo, Client wollte resource, yo", connection_id);
            None
        }

        TcpClientMessage::SubmitEdit {resource, new_data} => {
            r_log!(Info,"[Conn {}] Yo, der Nutzer ist fertig mit Resource, yo", connection_id);
            None
        }
    }
}