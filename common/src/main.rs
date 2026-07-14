mod fixture;
mod networking;

use std::io::Read;
use std::net::TcpStream;
use networking::messages::{TcpClientMessage, TcpServerMessage, UpdateMode, SubscribeTopic, UserRole};




///startPoint - This is the main entry point of the common application.
fn main() {
    println!("EDER stinkt!");
    use std::io::{self, Write};
    use std::env;

    let args: Vec<String> = env::args().collect();

    let target = if args.len() > 1 {
        &args[1]
    } else {
        "127.0.0.1:6767"
    };

    println!("[System] Versuche Verbindung zu {} aufzubauen...", target);

    let mut stream = TcpStream::connect(target)
        .expect("Verbindung fehlgeschlagen");

    loop {
        // println!();
        // println!("1 Connect");
        // println!("2 Subscribe Universes OnChange");
        // println!("3 Subscribe Universes Continuous");
        // println!("4 Subscribe FixturePositions OnChange");
        // println!("5 Subscribe FixturePositions Continuous");
        // println!("6 Unsubscribe Universes");
        // println!("7 Unsubscribe FixturePositions");
        // println!("8 Disconnect");
        // println!("0 Exit");

        print!("> ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        let msg = match input.trim() {
            "1" => TcpClientMessage::Connect {
                password: "".into(),
                user_name: "TestUser".into(),
                user_role: UserRole::Programmer,
            },

            "2" => TcpClientMessage::Subscribe {
                topic: SubscribeTopic::Universes,
                update_mode: UpdateMode::OnChange,
            },

            "3" => TcpClientMessage::Subscribe {
                topic: SubscribeTopic::Universes,
                update_mode: UpdateMode::Continuous,
            },

            "4" => TcpClientMessage::Subscribe {
                topic: SubscribeTopic::FixturePositions,
                update_mode: UpdateMode::OnChange,
            },

            "5" => TcpClientMessage::Subscribe {
                topic: SubscribeTopic::FixturePositions,
                update_mode: UpdateMode::Continuous,
            },

            "6" => TcpClientMessage::Unsubscribe {
                topic: SubscribeTopic::Universes,
            },

            "7" => TcpClientMessage::Unsubscribe {
                topic: SubscribeTopic::FixturePositions,
            },

            "8" => TcpClientMessage::Disconnect,

            "0" => break,

            command => TcpClientMessage::ExecuteCommand(command.into()),
        };

        let bytes = bincode::serialize(&msg).unwrap();
        stream.write_all(&bytes).unwrap();

        let mut response_buffer = [0; 1024];
        match stream.read(&mut response_buffer) {
            Ok(0) => {
                println!("[System] Server hat überraschend die Verbindung geschlossen.");
                break;
            }
            Ok(bytes_read) => {
                match bincode::deserialize::<TcpServerMessage>(&response_buffer[..bytes_read]) {
                    Ok(kernel_msg) => {
                        //println!("\n--- ANTWORT VOM SERVER ---");
                        //println!("Empfingenes Enum: {:?}", kernel_msg);

                        // Hier kannst du die Antwort noch hübsch matchen, falls gewünscht:
                        match kernel_msg {
                            TcpServerMessage::CommandOutput(output) => {
                                match output {
                                    Ok(response) => {
                                        println!("{}",response);
                                    }
                                    Err(e) => {
                                        println!("\x1b[31m{}\x1b[0m", e);
                                    }
                                }
                            }
                            TcpServerMessage::AssignUserID(token) => {
                                println!("Sitzung aktiv! Euer Token lautet: {}", token);
                            }
                            // Die anderen Varianten (TopicUpdate, etc.) analog...
                            _ => {}
                        }
                        // println!("--------------------------");
                    }
                    Err(e) => {
                        println!("[System] Fehler beim Deserialisieren der Serverantwort: {}", e);
                    }
                }
            }

            Err(e) => {
                println!("[System] Fehler beim Lesen vom Server-Socket: {}", e);
                break;
            }
        }
    }
}
