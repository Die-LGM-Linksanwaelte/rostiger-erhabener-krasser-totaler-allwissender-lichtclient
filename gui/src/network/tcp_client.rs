use crate::network::connection_state::ConnectionState;
use common::logging::LogLevel::{Error, Info, SuccessEvent};
use common::networking::messages::{
    HandshakeRequest, HandshakeResponse, TcpClientMessage, TcpServerMessage,
};
use common::{networking, r_log};
use std::env;
use std::io::{Read, Write};
use std::net::{Shutdown, TcpStream};
use std::sync::mpsc::{Receiver, Sender};
use std::thread;

pub(crate) struct TcpClient {
    target: String,
    tcp_receiver: Receiver<TcpClientMessage>,
    tcp_sender: Sender<TcpServerMessage>,
}

impl TcpClient {
    ///startPoint - This is the main entry point of the common application.
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

    ///sets the connection state by sending it to the controller
    fn set_connection_state(state: ConnectionState) {
        if let Some(sender) = crate::UI_EVENT_SENDER.read().unwrap().as_ref() {
            let _ = sender.send(crate::controller::UiEvent::SetConnectionState { state });
        }
    }

    ///initializes the tcp client and its read and write threads
    pub(crate) fn start_tcp_client(&mut self) {
        r_log!(
            Info,
            "Trying to establish connection to {} ...",
            self.target
        );

        let mut write_stream = match TcpStream::connect(&self.target) {
            Ok(stream) => stream,
            Err(e) => {
                eprintln!("[System] Unable to connect to {}: {}", self.target, e);
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
                Self::set_connection_state(ConnectionState::Connected);
            }
            HandshakeResponse::Mismatch { server_version } => {
                r_log!(
                    Error,
                    "Version mismatch! this client has the version {}. Kernel has the version {}",
                    client_version,
                    server_version
                );
                std::process::exit(1);
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

        thread::spawn(move || {
            Self::listen_tcp(read_stream, tcp_sender);
        });

        Self::write_thread(self, write_stream);
    }

    ///runs the listen thread that receives the tcp messages from the kernel and sends it to the controller
    fn listen_tcp(mut read_stream: TcpStream, tcp_sender: Sender<TcpServerMessage>) {
        loop {
            let mut len_buf = [0u8; 4];
            match read_stream.read_exact(&mut len_buf) {
                Ok(_) => {
                    let msg_len = u32::from_be_bytes(len_buf) as usize;
                    let mut response_buffer = vec![0u8; msg_len];

                    match read_stream.read_exact(&mut response_buffer){
                        Ok(_) => {
                            match bincode::deserialize::<TcpServerMessage>(&response_buffer) {
                                Ok(kernel_msg) => {
                                    if let Err(e) = tcp_sender.send(kernel_msg) {
                                        r_log!(Error, "Error sending TcpServerMessage: {}", e);
                                    }
                                }
                                Err(e) => {
                                    r_log!(Error, "Error deserializing TcpServerMessage: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            r_log!(Error, "Error reading from TcpStream: {}", e);
                            Self::set_connection_state(ConnectionState::Error);
                            break;
                        }
                    }
                }
                Err(e) => {
                    r_log!(Error, "Error reading from TcpStream: {}", e);
                    Self::set_connection_state(ConnectionState::Error);
                    break;
                }
            }
        }
    }

    ///runs the write thread, that receives the messages from the ui and sends it via tcp to the socket
    fn write_thread(&self, mut write_stream: TcpStream) {
        while let Ok(message) = self.tcp_receiver.recv() {
            match bincode::serialize(&message) {
                Ok(payload) => {
                    let len = payload.len() as u32;
                    if let Err(e) = write_stream.write_all(&len.to_be_bytes()) {
                        r_log!(Error, "Got error while sending length prefix: {} Stopped write-Thread", e);
                        break;
                    }
                    if let Err(e) = write_stream.write_all(&payload) {
                        r_log!(Error, "Got error while sending to Server: {} Stopped write-Thread", e);
                        break;
                    }
                }
                Err(e) => {
                    r_log!(Error, "Got error while serializing response: {}", e);
                    break;
                }
            }
        }
        //Thread the Ripper, we kill the read-thread with us
        let _ = write_stream.shutdown(Shutdown::Both);
        //r_log!(Info,"The Write-Thread {} is dead! Long live the Write-Thread!", connection_id);
    }
}
