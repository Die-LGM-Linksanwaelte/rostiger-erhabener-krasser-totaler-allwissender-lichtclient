use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::thread;
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver,Sender};
use std::time::Duration;
use rand::{RngExt};
use common::{r_log, r_debug_log};
use common::logging::LogLevel::*;
use common::networking::messages::{HandshakeRequest, HandshakeResponse, SessionID, TcpClientMessage, TcpServerMessage};
use common::networking::messages::TcpServerMessage::{CommandOutput, ImplicitCommandOutput, LogoutOk};
use common::networking::subscription_objects::UpdateMode;
use crate::networking::connection_engine::{ClientSession, ConnectionID, NEXT_CONNECTION_ID, SERVER_STATE};
use crate::networking::subscriptions::add_subscription;
use crate::cli::run_command;
use crate::cli::execute_implicit_cli_action;

/// Binds the TCP server socket to the specified port and spawns the main incoming connection listener loop.
///
/// If binding fails (e.g., because another instance is already running), an error is logged and the process exits.
/// Every accepted connection is assigned a unique [`ConnectionID`] and dispatched to its own dedicated worker thread.
///
/// # Arguments
///
/// * `port` - The network port number on which the server should listen for incoming client connections.
pub fn activate_socket(port: u16) {
    let address = format!("0.0.0.0:{}", port);
    let listener = match TcpListener::bind(&address) {
        Ok(l) => l,
        Err(e) => {
            r_log!(Error,"CRITICAL: Failed to bind TCP socket on address {}. Is another kernel instance already \
                    running? To change port, use --port [port]. OS Error: {}",address, e);
            // Give the Logger time to do its thing
            thread::sleep(Duration::from_millis(50));
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

                    thread::spawn(move || {
                        handle_client(stream, connection_id);
                    });
                }
                Err(e) => {
                    r_log!(Error,"Got error trying to establish connection : {}", e);
                }
            }
        }
    });
}

/// Orchestrates the lifecycle of an individual client connection.
///
/// Performs the initial protocol handshake and version compatibility check, establishes
/// independent channels for asynchronous communication, and spawns parallel reader and writer
/// threads for the stream.
///
/// # Arguments
///
/// * `stream`        - The active `TcpStream` associated with the connected client.
/// * `connection_id` - The unique identifier assigned to this physical connection.
fn handle_client(mut stream: TcpStream, connection_id: ConnectionID) {

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


    read_thread(&mut stream, connection_id, &tx_channel);

}

/// Represents possible errors that can occur during the client handshake phase.
enum HandshakeError {
    VersionMismatch {
        server_version: String,
        client_version: String,
    },
    InvalidData(String),
}

/// Validates the initial handshake packet sent by a newly connected client.
///
/// Verifies the custom magic string protocol identifier, deserializes the client's version details,
/// compares the protocol hash against the running server version, and transmits the appropriate
/// [`HandshakeResponse`] back across the stream.
///
/// # Arguments
///
/// * `stream` - A mutable reference to the client's `TcpStream`.
fn check_version_compatibility(stream: &mut TcpStream) -> Result<(), HandshakeError> {
    let mut len_buffer = [0u8; 4];
    let mut buffer = match stream.read_exact(&mut len_buffer) {
        Ok(_) => {
            let msg_len = u32::from_be_bytes(len_buffer) as usize;
            vec![0u8; msg_len]
        },
        Err(e) => {
            return Err(HandshakeError::InvalidData(format!("Len-Read Error: {}", e)));
        }
    };

    let bytes = match stream.read_exact(&mut buffer) {
        Err(e) => {
            return Err(HandshakeError::InvalidData(format!("Read Error: {}", e)));
        }
        Ok(_) => buffer
    };

    match bincode::deserialize::<HandshakeRequest>(&bytes) {
        Ok(request) => {
            if request.magic_string != "REKTAL" {
                return Err(HandshakeError::InvalidData("Wrong magic string. Client is not an Rektal-Client".into()));
            }

            let (server_hash, server_version) = common::networking::messages::get_protocol_version();

            if request.protocol_hash != server_hash {
                let response = HandshakeResponse::Mismatch {
                    server_version: server_version.clone(),
                };

                let payload = bincode::serialize(&response).unwrap();
                let len = payload.len() as u32;

                stream.write_all(&len.to_be_bytes()).unwrap();
                stream.write_all(&payload).unwrap();

                Err(HandshakeError::VersionMismatch {
                    server_version,
                    client_version: request.client_version
                })
            } else {
                let response = HandshakeResponse::Ok;
                let payload = bincode::serialize(&response).unwrap();
                let len = payload.len() as u32;
                stream.write_all(&len.to_be_bytes()).unwrap();
                stream.write_all(&payload).unwrap();

                Ok(())
            }

        }
        Err(_) => Err(HandshakeError::InvalidData("Couldnt deserialize handshake".into())),
    }

}

/// Continuously reads, deserializes, and processes incoming messages from an active client connection.
///
/// Handles session authorization enforcement, delegates requests to appropriate message handlers,
/// and performs cleanup operations (updating session status to sleeping) when the client disconnects.
///
/// # Arguments
///
/// * `stream`        - A mutable reference to the client's `TcpStream`.
/// * `connection_id` - The unique identifier of this connection.
/// * `tx_channel`    - The message sender channel used to push asynchronous server responses to the tcp writer thread.
fn read_thread(stream: &mut TcpStream, connection_id: ConnectionID, tx_channel: &Sender<TcpServerMessage>) {
    let mut len_buffer = [0u8; 4];
    let mut token: Option<SessionID> = None;

    loop {
        let mut buffer = match stream.read_exact(&mut len_buffer) {
            Ok(_) => {
                let msg_len = u32::from_be_bytes(len_buffer) as usize;
                vec![0u8; msg_len]
            },
            Err(e) => {
                if e.kind() == std::io::ErrorKind::UnexpectedEof {
                    if token.is_none() {
                        r_log!(SuccessEvent,"[Conn {}] Client disconnected successfully", connection_id);
                    } else {
                        r_log!(Error, "[Conn {}] Client disconnected without logging out. Session is still active",
                            connection_id
                        );
                    }
                } else {
                    r_log!(Error, "[Conn {}] Length of read-stream-error: {}", connection_id, e);
                }
                break;
            }
        };

        let bytes = match stream.read_exact(&mut buffer) {
            Err(e) => {
                r_log!(Error,"[Conn {}] Connection error : {}", connection_id, e);
                break;
            }
            Ok(_) => buffer
        };

        let msg = bincode::deserialize::<TcpClientMessage>(&bytes).unwrap();
        r_debug_log!(Info,"[Conn {}] Received Enum: {:?}", connection_id, msg);

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
                if let Some(response) = handle_messages(msg, connection_id, token.unwrap()) {
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
    r_debug_log!(Info,"The Read-Thread  {} is dead! Long live the Read-Thread!", connection_id);
}

/// Manages outgoing data serialization and transmission to the client on a dedicated background thread.
///
/// Listens on the receive channel for outbound [`TcpServerMessage`] objects, serializes them,
/// writes their length-prefixed binary representations to the TCP stream, and terminates upon
/// encountering transmission errors or a kick notification.
///
/// # Arguments
///
/// * `rx_channel`    - The receiver channel for pulling queued server-to-client messages.
/// * `write_stream`  - The isolated `TcpStream` clone dedicated to writing data.
/// * `connection_id` - The unique identifier of this connection.
fn write_thread(rx_channel: Receiver<TcpServerMessage>, mut write_stream: TcpStream, connection_id: ConnectionID) {
    while let Ok(message) = rx_channel.recv() {
        match bincode::serialize(&message) {
            Ok(serialized_response) => {
                let len = serialized_response.len() as u32;

                if let Err(e) = write_stream.write_all(&len.to_be_bytes()) {
                    r_log!(Warning,"[Conn {}] Got error while sending len to Client: {} Stopped write-Thread",
                             connection_id, e);
                    break;
                }

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
    r_debug_log!(Info,"The Write-Thread {} is dead! Long live the Write-Thread!", connection_id);
}

/// Evaluates authentication-related client messages (`Login`, `Relogin`, `Logout`) and updates global session states
/// (via return).
///
/// # Arguments
///
/// * `message`       - The incoming client message payload.
/// * `old_token`     - The existing session token associated with this connection context, if any.
/// * `tx_channel`    - The transmission channel to send immediate auth feedback.
/// * `connection_id` - The unique identifier of the connection.
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

            if password == "" {//TODO Save password somewhere and read it here
                let mut state = SERVER_STATE.write().unwrap();

                let mut rng = rand::rng();
                let new_token = loop {
                    let token:SessionID = rng.random();

                    if !state.contains_key(&token) {
                        break token;
                    }

                };

                state.insert(new_token, ClientSession {
                    _user_name: user_name.clone(),
                    _user_role: *user_role,
                    active_connection: Some((tx_channel.clone(), connection_id)),
                    subscriptions: vec![]
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
                    session.subscriptions.clear();
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
            r_log!(Error,"[Conn {}] Error sending the auth response to the channel: {}", connection_id, e);
        }
    }

    new_token
}

/// Routes general, authorized client messages to appropriate engine functions or command handlers.
///
/// # Arguments
///
/// * `msg`           - The deserialized command.
/// * `connection_id` - The unique identifier of the active connection.
/// * `token`         - The verified session identifier of the caller.
fn handle_messages(msg: TcpClientMessage, connection_id: ConnectionID, token: SessionID) -> Option<TcpServerMessage> {
    match msg {
        TcpClientMessage::Login {..} | TcpClientMessage::Logout | TcpClientMessage::Relogin {..} => unreachable!(),


        TcpClientMessage::Subscribe {topic, update_mode} => {
            add_subscription(&token, &topic, &update_mode);
            r_log!(Info,"[Conn {}] {}", connection_id, match update_mode {
                UpdateMode::OnChange => format!("Client requested updates on changes for {}!", topic),
                UpdateMode::Continuous => format!("Client requested continuous updates for {}", topic),
            }); //TODO Normally we always should respond to this
            None
        }

        TcpClientMessage::Unsubscribe { topic } => {
            r_log!(Info, "[Conn {}] Client unsubscribed from {}.", connection_id, topic);
            None
        }

        TcpClientMessage::ExecuteCommand{ command, response_id} => {
            let answer = run_command(false, command);
            r_log!(answer.0, "[Conn {}] {}", connection_id, answer.1);
            Some(CommandOutput{
                answer,
                response_id
            })
        }

        TcpClientMessage::ExecuteImplicitCommand { command, response_id} => {
            let answer = execute_implicit_cli_action(&command);
            //TODO Add logging for this
            Some(ImplicitCommandOutput {
                answer,
                response_id
            })
        }

        TcpClientMessage::RequestEdit(_) => {
            r_log!(Info, "[Conn {}] Client requested resource edit lock", connection_id);
            None
        }

        TcpClientMessage::SubmitEdit {resource:_, new_data:_} => {
            r_log!(Info,"[Conn {}] User submitted change to resource", connection_id);
            None
        },
        TcpClientMessage::DropEditLock(_) => {
            r_log!(Info, "[Conn {}] User dropped resource edit lock", connection_id);
            None
        }
    }
}