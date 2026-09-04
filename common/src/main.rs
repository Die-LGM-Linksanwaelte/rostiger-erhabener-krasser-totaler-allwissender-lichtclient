use std::io::{self, Read, Write};
use std::net::{Shutdown, TcpStream};
use std::thread;
use std::env;
use common::networking::messages::{TcpClientMessage, TcpServerMessage, UserRole, HandshakeRequest, HandshakeResponse};
use common::networking::subscription_objects::{SubscribeTopic, UpdateMode, TopicPayload, DMXConfigurationForClient};

/// Main entry point of the common application.
/// Used as a simple TUI-Client for testing features not yet implemented in the GUI.
fn main() {

    let args: Vec<String> = env::args().collect();
    let target = if args.len() > 1 {
        &args[1]
    } else {
        "127.0.0.1:6767"
    };

    println!("[System] Attempting to connect to {}...", target);

    let mut write_stream = TcpStream::connect(target)
        .expect("Connection failed");

    // ...
    let (protocol_hash, client_version) = common::networking::messages::get_protocol_version();

    let req = HandshakeRequest {
        magic_string: "REKTAL".into(),
        protocol_hash,
        client_version: client_version.clone(),
    };

    let payload = bincode::serialize(&req).unwrap();
    let len = payload.len() as u32;

    write_stream.write_all(&len.to_be_bytes()).unwrap();
    write_stream.write_all(&payload).unwrap();

    let mut len_buffer = [0u8; 4];


    let mut buffer = match write_stream.read_exact(&mut len_buffer) {
        Ok(_) => {
            let msg_len = u32::from_be_bytes(len_buffer) as usize;
            vec![0u8; msg_len]
        },
        Err(e) => {
            println!("In Establishing: Length of read-stream-error: {}", e);
            return;
        }
    };

    let bytes = match write_stream.read_exact(&mut buffer) {
        Err(e) => {
            println!("In Establishing: Connection error : {}", e);
            return;
        }
        Ok(_) => buffer
    };

    let res = match bincode::deserialize::<HandshakeResponse>(&bytes) {
        Ok(res) => res,
        Err(e) => {
            println!("\n[System] Error deserializing handshake: {}", e);
            return;
        }
    };

    match res {
        HandshakeResponse::Ok => {
            println!("\x1b[32m[System] Version {} verified!\x1b[0m", client_version);
        }
        HandshakeResponse::Mismatch { server_version } => {

            // HERE IS YOUR ABSOLUTELY PERFECT ERROR MESSAGE:
            println!("\n\x1b[91m[CRITICAL ERROR] Protocol mismatch detected!\x1b[0m");
            println!("\x1b[93mThe Kernel is running on version: {}\x1b[0m", server_version);
            println!("\x1b[93mThis client is on version: {}\x1b[0m", client_version);

            // The smart decision helper for the user:
            println!("GPlus are you dumb, you can't compare strings with < or >!");
            // if client_version < server_version {
            //      println!("→ Please download the latest client!");
            // } else {
            //      println!("→ You are using a client from the future. Please update the Kernel on the lighting console!");
            // }

            std::process::exit(1);
        }
    }

    // THE CLONE TRICK FOR THE CLIENT
    let mut read_stream = write_stream.try_clone().expect("Could not clone stream");

    // ---------------------------------------------------------
    // READ THREAD (Background)
    // ---------------------------------------------------------
    thread::spawn(move || {
        loop {
            let mut len_buffer = [0u8; 4];
            let mut buffer = match read_stream.read_exact(&mut len_buffer) {
                Ok(_) => {
                    let msg_len = u32::from_be_bytes(len_buffer) as usize;
                    vec![0u8; msg_len]
                },
                Err(e) => {
                    if e.kind() == io::ErrorKind::UnexpectedEof {
                        println!("Server was terminated");
                        read_stream.shutdown(Shutdown::Both).expect("Shutdown failed");
                        std::process::exit(0);
                    }
                    println!("[Read-stream] Len-Error: {}", e);
                    break;
                }
            };

            match read_stream.read_exact(&mut buffer) {
                Err(e) => {
                    println!("[Read-Stream] Connection error : {}", e);
                    break;
                }
                Ok(_) => {
                    match bincode::deserialize::<TcpServerMessage>(&buffer) {
                        Ok(kernel_msg) => {
                            // We insert a line break beforehand so the "> " prompt doesn't get torn apart
                            print!("\r\x1b[2K"); // Briefly clears the current input line for clean output

                            match kernel_msg {
                                TcpServerMessage::Unauthenticated => {
                                    println!("\n\x1b[31m[Server] Client is not authenticated. Message discarded\x1b[0m");
                                }
                                TcpServerMessage::LoginOk { token } => {
                                    println!("\n\x1b[32m[System] Login successful! Your session token: {}\x1b[0m", token);
                                }
                                TcpServerMessage::LoginFailed { reason } => {
                                    println!("\n\x1b[31m[System] Login failed: {}\x1b[0m", reason);
                                }
                                TcpServerMessage::ReloginOk { token } => {
                                    println!("\n\x1b[32m[System] Relogin successful! Session {} active.\x1b[0m", token);
                                }
                                TcpServerMessage::ReloginFailed { reason } => {
                                    println!("\n\x1b[31m[System] Relogin failed: {}\x1b[0m", reason);
                                }
                                TcpServerMessage::LogoutOk => {
                                    println!("\n\x1b[34m[System] Successfully logged out.\x1b[0m");
                                }
                                TcpServerMessage::Kicked { reason } => {
                                    println!("\n\x1b[31m[System] You have been kicked: {}\x1b[0m", reason);
                                    std::process::exit(0); // Exit client
                                }
                                TcpServerMessage::CommandOutput{answer: result, ..} => {
                                    println!("[{}] {}", result.0, result.1);
                                }

                                TcpServerMessage::TopicUpdate {data} => {
                                    match data {
                                        TopicPayload::DMXConfiguration(config) => {
                                            println!("\n\x1b[33m[System] Configuration updated\x1b[0m");
                                            for universe in config {
                                                for channel in universe {
                                                    match channel {
                                                        DMXConfigurationForClient::Empty => print!("."),
                                                        DMXConfigurationForClient::Reserved {
                                                            fixture_name, property_type,
                                                            fine_degree, fixture_type_hash: _
                                                        } => {
                                                            print!("|{}:{}:{}|",
                                                                   fixture_name, property_type, fine_degree
                                                            );
                                                        }
                                                    }
                                                }
                                                println!();
                                            }
                                        }
                                    }
                                }
                                other_msg => {
                                    println!("[Server] Displaying this server message is not yet implemented.\
                                     Trying anyway: {:?}", other_msg);
                                }
                            }

                            // Displays the prompt again afterwards
                            print!("> ");
                            io::stdout().flush().unwrap();
                        }
                        Err(e) => {
                            println!("\n[System] Error deserializing server response: {}", e);
                        }
                    }
                }
            }
        }
    });


    // ---------------------------------------------------------
    // INPUT LOOP (Main Thread)
    // ---------------------------------------------------------
    loop {
        print!("> ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        let msg = match input.trim() {
            "1" => TcpClientMessage::Login {
                password: "".into(), // Your password here
                user_name: "TestUser".into(),
                user_role: UserRole::Programmer,
            },

            "2" => TcpClientMessage::Subscribe {
                topic: SubscribeTopic::DMXConfiguration,
                update_mode: UpdateMode::OnChange,
            },

            "3" => TcpClientMessage::Subscribe {
                topic: SubscribeTopic::DMXConfiguration,
                update_mode: UpdateMode::Continuous,
            },

            "6" => TcpClientMessage::Unsubscribe {
                topic: SubscribeTopic::DMXConfiguration,
            },

            "8" => TcpClientMessage::Logout,

            "9" => {
                println!("\x1b[31m[System] SIMULATING CABLE BREAK! Going to sleep...\x1b[0m");
                // We ignore everything and freeze the main thread.
                // The server thinks we are still there, but we stop responding to anything.
                thread::sleep(std::time::Duration::from_secs(3600));
                break;
            }

            // The new relogin button
            "11" => {
                print!("Please enter session token: ");
                io::stdout().flush().unwrap();

                let mut token_str = String::new();
                io::stdin().read_line(&mut token_str).unwrap();

                match token_str.trim().parse::<u64>() {
                    Ok(token) => {
                        // We build the relogin message and send it off
                        TcpClientMessage::Relogin {
                            user_id: token,
                            clear_subscriptions: false,
                        }
                    }
                    Err(_) => {
                        println!("\x1b[31m[System] Invalid token. Please enter a number.\x1b[0m");
                        continue; // Aborts this iteration and displays the "> " prompt again
                    }
                }
            }

            "0" => break, // Ends the loop and thus the program

            command => TcpClientMessage::ExecuteCommand {command: command.into(), response_id: 0},
        };

        // Send message, thread NO LONGER blocks waiting for an answer here!
        if let Ok(_bytes) = bincode::deserialize::<Vec<u8>>(&bincode::serialize(&msg).unwrap()) {
            // Just a little safety net against broken Serialize calls.
            // Better to do it directly:
        }

        let bytes = bincode::serialize(&msg).unwrap();
        let bytes_len = bytes.len() as u32;

        if write_stream.write_all(&bytes_len.to_be_bytes()).is_err() {
            println!("[System] Could not send len. Connection dead?");
            break;
        }

        if write_stream.write_all(&bytes).is_err() {
            println!("[System] Could not send. Connection dead?");
            break;
        }
    }
}