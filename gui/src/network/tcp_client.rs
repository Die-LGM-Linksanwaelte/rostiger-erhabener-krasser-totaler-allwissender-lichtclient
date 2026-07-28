use crate::network::connection_state::ConnectionState;
use crate::panels::terminal::TerminalPanel;
use crate::panels::Tab::Terminal;
use crate::{MyApp /*TCP_SENDER*/};
use common::networking;
use common::networking::messages::{
    HandshakeRequest, HandshakeResponse, SubscribeTopic, TcpClientMessage, TcpServerMessage,
    UpdateMode, UserRole,
};
use std::env;
use std::io::{self, Read, Write};
use std::net::{Shutdown, TcpStream};
use std::sync::mpsc::{Receiver, Sender};
use std::thread;

pub(crate) struct TcpClient {
    target: String,
    connection_state: ConnectionState,
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
            connection_state: ConnectionState::Disconnected,
            tcp_receiver,
            tcp_sender,
        }
    }
    pub(crate) fn start_tcp_client(&mut self) {
        //TODO refractoring
        println!("EDER stinkt!");

        println!(
            "[System] Trying to establish connection to {} ...",
            self.target
        );

        let mut write_stream =
            TcpStream::connect(self.target.clone()).expect("Connection couldn't be established!");

        let client_version = env!("CARGO_PKG_VERSION");
        let protocol_hash = networking::messages::get_protocol_version();

        let req = HandshakeRequest {
            magic_string: "REKTAL".into(),
            protocol_hash,
            client_version: client_version.into(),
        };

        self.connection_state = ConnectionState::ConnectionPending;

        write_stream
            .write_all(&bincode::serialize(&req).unwrap())
            .unwrap(); //TODO fehlerbehandlung

        let mut buffer = [0; 1024];

        let bytes = match write_stream.read(&mut buffer) {
            Ok(0) => {
                println!("\x1b[91m[Connection was terminated instantly\x1b[0m");
                self.connection_state = ConnectionState::Disconnected;
                return;
            }
            Ok(b) => b,
            Err(e) => {
                println!("\x1b[91m[Read-Error: {}\x1b[0m", e);
                self.connection_state = ConnectionState::Error;
                return;
            }
        };

        let res = match bincode::deserialize::<HandshakeResponse>(&buffer[..bytes]) {
            Ok(res) => res,
            Err(e) => {
                println!(
                    "\n[System] Fehler beim Deserialisieren des Handshakes: {}",
                    e
                );
                return;
            }
        };

        match res {
            HandshakeResponse::Ok => {
                println!(
                    "\x1b[32m[System] Version {} verifiziert!\x1b[0m",
                    client_version
                );
            }
            HandshakeResponse::Mismatch { server_version } => {
                // HIER IST DEINE ABSOLUT PERFEKTE FEHLERMELDUNG:
                println!("\n\x1b[91m[CRITICAL ERROR] Protokoll-Abweichung erkannt!\x1b[0m");
                println!(
                    "\x1b[93mDer Kernel läuft auf Version: {}\x1b[0m",
                    server_version
                );
                println!(
                    "\x1b[93mDieser Client ist auf Version: {}\x1b[0m",
                    client_version
                );

                std::process::exit(1);
            }
        }

        let mut read_stream = write_stream
            .try_clone()
            .expect("Konnte Stream nicht klonen");

        let tcp_sender = self.tcp_sender.clone();

        thread::spawn(move || {
            Self::listen_tcp(read_stream, tcp_sender);
        });

        Self::write_thread(self, write_stream);
    }

    fn listen_tcp(mut read_stream: TcpStream, tcp_sender: Sender<TcpServerMessage>) {
        let mut response_buffer = [0; 4096]; // Etwas größerer Buffer schadet nie

        loop {
            match read_stream.read(&mut response_buffer) {
                Ok(0) => {
                    println!("\n[System] Server hat die Verbindung geschlossen.");
                    // Beendet das gesamte Programm, wenn der Server weg ist
                    std::process::exit(0);
                }
                Ok(bytes_read) => {
                    match bincode::deserialize::<TcpServerMessage>(&response_buffer[..bytes_read]) {
                        Ok(kernel_msg) => {
                            // Einfach an den Main-Thread (MyApp) weiterleiten!
                            if let Err(e) = tcp_sender.send(kernel_msg) {
                                eprintln!("Fehler beim Weiterleiten der Netzwerk-Nachricht an die GUI: {}", e);
                            }
                        }
                        Err(e) => {
                            println!(
                                "\n[System] Fehler beim Deserialisieren der Serverantwort: {}",
                                e
                            );
                        }
                    }
                }
                Err(e) => {
                    println!("\n[System] Fehler beim Lesen vom Server-Socket: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }

    fn write_thread(&self, mut write_stream: TcpStream) {
        while let Ok(message) = self.tcp_receiver.recv() {
            match bincode::serialize(&message) {
                Ok(serialized_response) => {
                    if let Err(e) = write_stream.write_all(&serialized_response) {
                        //r_log!(Warning,"[Conn {}] Got error while sending to Client: {} Stopped write-Thread", connection_id, e);
                        break;
                    }
                }

                Err(e) => {
                    //r_log!(Error,"[Conn {}] Got error while serializing response: {}", connection_id, e);
                    break;
                }
            }
        }

        //Thread the Ripper, we kill the read-thread with us
        let _ = write_stream.shutdown(Shutdown::Both);
        //r_log!(Info,"The Write-Thread {} is dead! Long live the Write-Thread!", connection_id);
    }
}
