//! # TCP Client Module
//!
//! This module manages the TCP network connection from the GUI to the kernel server.
//! It handles the initial handshake, asynchronous transmission of UI commands, 
//! and continuous listening for responses (like logs or subscriptions) 
//! using dedicated background threads.

use crate::network::connection_state::{ConnectionState, SessionState};
use common::logging::LogLevel::{Error, Info, SuccessEvent};
use common::networking::messages::{
    HandshakeRequest, HandshakeResponse, TcpClientMessage, TcpServerMessage,
};
use common::{networking, r_log};
use std::env;
use std::io::{Read, Write};
use std::net::{Shutdown, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::thread;
use crate::controller::{send_ui_event, UiEvent};

/// Manages a persistent TCP connection to the kernel server.
///
/// The `TcpClient` encapsulates stream handles and channels. Upon starting, it spawns 
/// separate background threads for non-blocking reading and writing of network messages.
pub(crate) struct TcpClient {
    target: String,
    tcp_receiver: Receiver<TcpClientMessage>,
    tcp_sender: Sender<TcpServerMessage>,
}

impl TcpClient {
    /// Creates a new [`TcpClient`] instance without starting it immediately.
    ///
    /// # Arguments
    /// * `target` - The IP address or hostname of the target server (e.g., "127.0.0.1:6767").
    /// * `tcp_receiver` - Channel to receive outgoing messages from the UI.
    /// * `tcp_sender` - Channel to forward incoming messages from the server to the UI controller.
    pub(crate) fn new(
        target: String,
        tcp_receiver: Receiver<TcpClientMessage>,
        tcp_sender: Sender<TcpServerMessage>,
    ) -> Self {
        Self {
            target,
            tcp_receiver,
            tcp_sender,
        }
    }

    /// Helper function to report a connection state change to the GUI controller.
    ///
    /// Uses the global `UI_EVENT_SENDER` to trigger a [`UiEvent::SetConnectionState`].
    ///
    /// # Arguments
    /// * `state` - The new connection state to apply in the UI.
    fn set_connection_state(state: ConnectionState) {
        if let Some(sender) = crate::UI_EVENT_SENDER.read().unwrap().as_ref() {
            let _ = sender.send(UiEvent::SetConnectionState { state });
        }
    }

    /// Starts the TCP client, performs the version handshake, and spawns the IO threads.
    ///
    /// Establishes the TCP connection and sends a `HandshakeRequest`.
    /// On a version mismatch (`HandshakeResponse::Mismatch`), the function aborts
    /// and sets the GUI to an `Error` state. Otherwise, the read and write threads 
    /// are started in the background.
    pub(crate) fn start_tcp_client(&mut self) {
        r_log!(
            Info,
            "Trying to establish connection to {} ...",
            self.target
        );

        let mut write_stream = match TcpStream::connect(&self.target) {
            Ok(stream) => stream,
            Err(e) => {
                r_log!(Error, "Unable to connect to {}: {}", self.target, e);
                return;
            }
        };

        let client_version = env!("CARGO_PKG_VERSION");
        let protocol_hash = networking::messages::get_protocol_version();

        let req = HandshakeRequest {
            magic_string: "REKTAL".into(),
            protocol_hash,
            client_version: client_version.into(),
        };

        Self::set_connection_state(ConnectionState::ConnectionPending);

        match bincode::serialize(&req) {
            Ok(payload) => {
                let len = payload.len() as u32;
                if let Err(e) = write_stream.write_all(&len.to_be_bytes()) {
                    r_log!(Error, "Got error while sending length prefix: {}", e);
                    return;
                }
                if let Err(e) = write_stream.write_all(&payload) {
                    r_log!(Error, "Got error while sending HandshakeRequest: {}", e);
                    return;
                }
            }
            Err(e) => {
                r_log!(Error, "Got error while serializing HandshakeRequest: {}", e);
                return;
            }
        }

        // Read HandshakeResponse with length prefix
        let mut len_buf = [0u8; 4];
        if let Err(e) = write_stream.read_exact(&mut len_buf) {
            r_log!(Error, "Error reading HandshakeResponse length: {}", e);
            Self::set_connection_state(ConnectionState::Error);
            return;
        }
        let msg_len = u32::from_be_bytes(len_buf) as usize;
        let mut response_buffer = vec![0u8; msg_len];
        if let Err(e) = write_stream.read_exact(&mut response_buffer) {
            r_log!(Error, "Error reading HandshakeResponse body: {}", e);
            Self::set_connection_state(ConnectionState::Error);
            return;
        }

        let res = match bincode::deserialize::<HandshakeResponse>(&response_buffer) {
            Ok(res) => res,
            Err(e) => {
                r_log!(Error, "Error deserializing HandshakeResponse: {}", e);
                return;
            }
        };

        match res {
            HandshakeResponse::Ok => {
                r_log!(SuccessEvent, "Version {} verified!", client_version);
                Self::set_connection_state(ConnectionState::Connected {
                    session_state: SessionState::LoggedOut,
                });
            }
            HandshakeResponse::Mismatch { server_version } => {
                r_log!(
                    Error,
                    "Version mismatch! this client has the version {}. Kernel has the version {}",
                    client_version,
                    server_version
                );
                Self::set_connection_state(ConnectionState::Error);
                return;
            }
        }

        let read_stream = match write_stream.try_clone() {
            Ok(stream) => stream,
            Err(e) => {
                r_log!(Error, "Failed to clone TcpStream: {}", e);
                return;
            }
        };

        let tcp_sender = self.tcp_sender.clone();
        let is_disconnecting = Arc::new(AtomicBool::new(false));
        let is_disconnecting_clone = Arc::clone(&is_disconnecting);

        thread::spawn(move || {
            Self::listen_tcp(read_stream, tcp_sender, is_disconnecting_clone);
        });

        Self::write_thread(self, write_stream, is_disconnecting);
    }

    /// Background thread: Continuously reads incoming messages from the server.
    ///
    /// Blocks while waiting for data, reads the 4-byte length prefix, loads the payload, 
    /// deserializes the [`TcpServerMessage`], and forwards it to the UI controller 
    /// via the `tcp_sender`.
    fn listen_tcp(
        mut read_stream: TcpStream,
        tcp_sender: Sender<TcpServerMessage>,
        is_disconnecting: Arc<AtomicBool>,
    ) {
        loop {
            let mut len_buf = [0u8; 4];
            let mut response_buffer = match read_stream.read_exact(&mut len_buf) {
                Ok(_) => {
                    let msg_len = u32::from_be_bytes(len_buf) as usize;
                    vec![0u8; msg_len]
                }
                Err(e) => {
                    if is_disconnecting.load(Ordering::SeqCst) {
                        r_log!(Info, "TCP connection closed (requested by client).");
                    } else {
                        r_log!(Error, "TCP connection lost unexpectedly from server: {}", e);
                        Self::set_connection_state(ConnectionState::Error);
                    }
                    break;
                }
            };

            match read_stream.read_exact(&mut response_buffer) {
                Ok(_) => match bincode::deserialize::<TcpServerMessage>(&response_buffer) {
                    Ok(kernel_msg) => {
                        if let Err(e) = tcp_sender.send(kernel_msg) {
                            if is_disconnecting.load(Ordering::SeqCst) {
                                r_log!(Info, "TCP receiver channel closed, stopping listen thread.");
                            } else {
                                r_log!(Error, "Error sending TcpServerMessage: {}", e);
                            }
                            break;
                        }
                    }
                    Err(e) => {
                        r_log!(Error, "Error deserializing TcpServerMessage: {}", e);
                    }
                },
                Err(e) => {
                    if is_disconnecting.load(Ordering::SeqCst) {
                        r_log!(Info, "TCP connection closed while reading body.");
                    } else {
                        r_log!(Error, "Error reading from TcpStream: {}", e);
                        Self::set_connection_state(ConnectionState::Error);
                    }
                    break;
                }
            }
        }
    }

    /// Background thread: Waits for outgoing messages from the GUI and sends them.
    ///
    /// Receives messages from the `tcp_receiver` channel (blocking), serializes them 
    /// into bytes, and sends them with a length prefix over the outgoing TCP stream to the kernel.
    fn write_thread(&self, mut write_stream: TcpStream, is_disconnecting: Arc<AtomicBool>) {
        while let Ok(message) = self.tcp_receiver.recv() {
            match bincode::serialize(&message) {
                Ok(payload) => {
                    let len = payload.len() as u32;
                    if let Err(e) = write_stream.write_all(&len.to_be_bytes()) {
                        r_log!(
                            Error,
                            "Got error while sending length prefix: {} Stopped write-Thread",
                            e
                        );
                        break;
                    }
                    if let Err(e) = write_stream.write_all(&payload) {
                        r_log!(
                            Error,
                            "Got error while sending to Server: {} Stopped write-Thread",
                            e
                        );
                        break;
                    }
                }
                Err(e) => {
                    r_log!(Error, "Got error while serializing response: {}", e);
                    break;
                }
            }
        }
        is_disconnecting.store(true, Ordering::SeqCst);
        Self::close_tcp_connection(&write_stream);
    }

    /// Closes the TCP connection cleanly in both directions.
    ///
    /// Using `shutdown(Shutdown::Both)` immediately interrupts traffic and 
    /// unblocks the reading thread so it can terminate gracefully.
    fn close_tcp_connection(stream: &TcpStream) {
        if let Err(e) = stream.shutdown(Shutdown::Both) {
            r_log!(Error, "Failed to shutdown TcpStream: {}", e);
        }
        send_ui_event(UiEvent::SetConnectionState {
            state: ConnectionState::Disconnected,
        });
    }
}