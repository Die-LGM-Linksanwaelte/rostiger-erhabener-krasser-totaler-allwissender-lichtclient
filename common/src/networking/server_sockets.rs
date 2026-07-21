use crate::networking::messages::TcpServerMessage::CommandOutput;
use crate::networking::messages::{
    SubscribeTopic, TcpClientMessage, TcpServerMessage, UpdateMode, UserRole,
};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

pub fn activate_socket<F>(port: u16, command_handler: F)
where
    F: Fn(String) -> Result<String, String> + Send + Sync + 'static + Clone,
{
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).expect("Failed to bind socket");
    println!(
        "\x1b[34mListening on {} for Clients\x1b[0m",
        listener.local_addr().unwrap()
    );

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!(
                    "\x1b[34mNew client connected: {}\x1b[0m",
                    stream.peer_addr().unwrap()
                );

                let handler_clone = command_handler.clone();

                thread::spawn(move || {
                    handle_client(stream, handler_clone);
                });
            }
            Err(e) => {
                println!(
                    "\x1b[91mGot error trying to establish connection : {}\x1b[0m",
                    e
                );
            }
        }
    }
}

fn handle_client<F>(mut stream: TcpStream, command_handler: F)
where
    F: Fn(String) -> Result<String, String> + Send + Sync + 'static + Clone,
{
    let mut buffer = [0; 1024];

    loop {
        match stream.read(&mut buffer) {
            Ok(0) => {
                println!("\x1b[34mClient disconnected successfully\x1b[0m");
                break;
            }
            Ok(bytes) => match bincode::deserialize::<TcpClientMessage>(&buffer[..bytes]) {
                Ok(msg) => {
                    println!("\x1b[34mEmpfangenes Enum: {:?}\x1b[0m", msg);
                    let response = handle_messages(msg, command_handler.clone());
                    if let Some(response) = response {
                        respond(&mut stream, response);
                    }
                }
                Err(e) => {
                    println!("\x1b[91mGot error de-serializing bincode : {}\x1b[0m", e);
                }
            },
            Err(e) => {
                println!("\x1b[91mConnection error : {}\x1b[0m", e);
                break;
            }
        }
    }
}

fn respond(stream: &mut TcpStream, response: TcpServerMessage) {
    match bincode::serialize(&response) {
        Ok(serialized_response) => {
            if let Err(e) = stream.write(&serialized_response) {
                println!("\x1b[91mGot error while serializing response: {}\x1b[0m", e);
            }
        }

        Err(e) => {
            println!("\x1b[91mGot error while serializing response: {}\x1b[0m", e);
        }
    }
}

fn handle_messages<F>(msg: TcpClientMessage, command_handler: F) -> Option<TcpServerMessage>
where
    F: Fn(String) -> Result<String, String> + Send + Sync + 'static + Clone,
{
    match msg {
        TcpClientMessage::Connect {
            password,
            user_name,
            user_role,
        } => {
            let role_name = match user_role {
                UserRole::Programmer => "Programmer",
                UserRole::BlindProgrammer => "Blind Programmer",
                UserRole::Showrunner => "Showrunner",
            };
            println!(
                "\x1b[34mDer Nutzer {} hat sich als {} mit Passwort {} verbunden!\x1b[0m",
                user_name, role_name, password
            );
            None
            //TODO Assign user_id
        }

        TcpClientMessage::Disconnect => {
            println!("\x1b[34mClient will weg\x1b[0m");
            None
        }

        TcpClientMessage::Reconnect {
            password,
            user_id,
            clear_subscriptions,
        } => {
            if clear_subscriptions {
                println!(
                    "\x1b[34mDer Nutzer mir der ID {user_id} hat sich mit Passwort {password} neu verbunden und\
                 will alte Subscriptions löschen.\x1b[0m"
                )
            } else {
                println!(
                    "\x1b[34mDer Nutzer mir der ID {user_id} hat sich mit Passwort {password} neu verbunden und\
                 will alte Subscriptions beibehalten.\x1b[0m"
                )
            }
            None
        }

        TcpClientMessage::Subscribe { topic, update_mode } => {
            let topic_name = match topic {
                SubscribeTopic::FixturePositions => "Fixture Positions",
                SubscribeTopic::Universes => "Universes",
            };
            println!(
                "\x1b[34m{}\x1b[0m",
                match update_mode {
                    UpdateMode::OnChange => format!(
                        "Der Client will über Änderungen von {} erfahren!",
                        topic_name
                    ),
                    UpdateMode::Continuous => format!(
                        "Der Client will über {} auf dem laufenden gehalten werden",
                        topic_name
                    ),
                }
            ); //TODO Normally we always should respond to this
            None
        }

        TcpClientMessage::Unsubscribe { topic } => {
            println!(
                "\x1b[34mDer Client will nichts von {} wissen.\x1b[0m",
                match topic {
                    SubscribeTopic::FixturePositions => "Fixture Positions",
                    SubscribeTopic::Universes => "Universes",
                }
            );
            None
        }

        TcpClientMessage::ExecuteCommand(command) => {
            let response = command_handler(command);
            match response.clone() {
                Ok(response_ok) => {
                    println!("\x1b[92m{}\x1b[0m", response_ok);
                }
                Err(response_error) => {
                    println!("\x1b[93m{}\x1b[0m", response_error);
                }
            }
            Some(CommandOutput(response))
        }

        TcpClientMessage::RequestEdit(_) => {
            println!("\x1b[34mYo, Client wollte resource, yo\x1b[0m");
            None
        }

        TcpClientMessage::SubmitEdit { resource, new_data } => {
            println!("\x1b[34mYo, der Nutzer ist fertig mit Resource, yo\x1b[0m");
            None
        }
    }
}
