use std::io::Read;
use std::net::{TcpListener, TcpStream};
use std::{thread};
use crate::networking::messages::{SubscribeTopic, TcpClientMessage, UpdateMode, UserRole};

pub fn activate_socket() {
    let listener = TcpListener::bind("0.0.0.0:6767").expect("Failed to bind socket");
    println!("Listening on {} for Clients", listener.local_addr().unwrap());

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("New client connected: {}", stream.peer_addr().unwrap());

                thread::spawn(move|| {
                    handle_client(stream);
                });
            }
            Err(e) => {
                println!("Got error trying to establish connection : {}", e);
            }
        }
    }
}

fn handle_client(mut stream: TcpStream) {
    let mut buffer = [0; 1024];

    loop {
        match stream.read(&mut buffer) {
            Ok(0) => {
                println!("Client disconnected successfully");
                break;
            }
            Ok(bytes) => {
                match bincode::deserialize::<TcpClientMessage>(&buffer[..bytes]) {
                    Ok(msg) => {
                        println!("Erfolgreich de-serialisiert!");
                        println!("Empfangenes Enum: {:?}", msg);
                        handle_messages(msg);
                    }
                    Err(e) => {
                        println!("Got error de-serializing bincode : {}", e);
                    }
                }
                println!("Client sent {} bytes", bytes);
                let text = String::from_utf8_lossy(&buffer[..bytes]);
                println!("Nachricht vom Client: '{}'", text.trim());
            }
            Err(e) => {
                println!("Connection error : {}", e);
                break;
            }
        }
    }
}

fn handle_messages(msg: TcpClientMessage) {
    match msg {
        TcpClientMessage::Connect {user_id, user_name, user_role} => {
            let role_name = match user_role {
                UserRole::Programmer => "Programmer",
                UserRole::BlindProgrammer => "Blind Programmer",
                UserRole::Showrunner => "Showrunner",
                UserRole::Interface => "Interface",
            };
            println!("Der Nutzer {} mit der ID {} hat sich als {} verbunden!", user_name, user_id, role_name);
        }

        TcpClientMessage::Disconnect => {
            println!("Client will weg");
        }

        TcpClientMessage::Subscribe {topic, update_mode} => {
            let topic_name = match topic {
                SubscribeTopic::FixturePositions => "Fixture Positions",
                SubscribeTopic::Universes => "Universes",
            };
            println!("{}", match update_mode {
                UpdateMode::OnChange => format!("Der Client will über Änderungen von {} erfahren!", topic_name),
                UpdateMode::Continuous => format!("Der Client will über {} auf dem laufenden gehalten werden", topic_name),
            });
        }

        TcpClientMessage::Unsubscribe { topic } => {
            println!("Der Client will nichts von {} wissen.", match topic {
                SubscribeTopic::FixturePositions => "Fixture Positions",
                SubscribeTopic::Universes => "Universes",
            });
        }
    }
}