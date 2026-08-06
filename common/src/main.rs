use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::thread;
use std::env;
use common::networking::messages::{TcpClientMessage, TcpServerMessage, UserRole, HandshakeRequest, HandshakeResponse};
use common::networking::{SubscribeTopic, UpdateMode};

///startPoint - This is the main entry point of the common application.
fn main() {
    println!("EDER stinkt!");

    let args: Vec<String> = env::args().collect();
    let target = if args.len() > 1 {
        &args[1]
    } else {
        "127.0.0.1:6767"
    };

    println!("[System] Versuche Verbindung zu {} aufzubauen...", target);

    let mut write_stream = TcpStream::connect(target)
        .expect("Verbindung fehlgeschlagen");

    // ...
    let client_version = env!("CARGO_PKG_VERSION");
    let protocol_hash = common::networking::messages::get_protocol_version();

    let req = HandshakeRequest {
        magic_string: "REKTAL".into(),
        protocol_hash,
        client_version: client_version.into(),
    };

    write_stream.write_all(&bincode::serialize(&req).unwrap()).unwrap();

    let mut buffer = [0; 1024];

    let bytes = match write_stream.read(&mut buffer) {
        Ok(0) => {
            println!("\x1b[91m[Connection was terminated instantly\x1b[0m");
            return;
        }
        Ok(b) => b,
        Err(e) => {
            println!("\x1b[91m[Read-Error: {}\x1b[0m", e);
            return;
        }
    };

    let res = match bincode::deserialize::<HandshakeResponse>(&buffer[..bytes]) {
        Ok(res) => res,
        Err(e) => {
            println!("\n[System] Fehler beim Deserialisieren des Handshakes: {}", e);
            return;
        }
    };

    match res {
        HandshakeResponse::Ok => {
            println!("\x1b[32m[System] Version {} verifiziert!\x1b[0m", client_version);
        }
        HandshakeResponse::Mismatch { server_version } => {

            // HIER IST DEINE ABSOLUT PERFEKTE FEHLERMELDUNG:
            println!("\n\x1b[91m[CRITICAL ERROR] Protokoll-Abweichung erkannt!\x1b[0m");
            println!("\x1b[93mDer Kernel läuft auf Version: {}\x1b[0m", server_version);
            println!("\x1b[93mDieser Client ist auf Version: {}\x1b[0m", client_version);

            // Die smarte Entscheidungshilfe für den User:
            println!("GPlus bist du dumm, man kann Strings nicht mit < oder > vergleichen!");
            // if client_version < server_version {
            //     println!("→ Bitte lade dir den neuesten Client herunter!");
            // } else {
            //     println!("→ Du nutzt einen Client aus der Zukunft. Bitte update den Kernel auf dem Lichtpult!");
            // }

            std::process::exit(1);
        }
    }

    // DER KLON-TRICK FÜR DEN CLIENT
    let mut read_stream = write_stream.try_clone().expect("Konnte Stream nicht klonen");

    // ---------------------------------------------------------
    // LESE-THREAD (Hintergrund)
    // ---------------------------------------------------------
    thread::spawn(move || {
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
                            // Wir machen einen Zeilenumbruch vorher, damit das "> " vom Prompt nicht zerrissen wird
                            print!("\r\x1b[2K"); // Löscht die aktuelle Eingabezeile kurz für sauberen Output

                            match kernel_msg {
                                TcpServerMessage::Unauthenticated => {
                                    println!("\n\x1b[31m[Server] Client ist nicht angemeldet. Nachricht verworfen\x1b[0m");
                                }
                                TcpServerMessage::LoginOk { token } => {
                                    println!("\n\x1b[32m[System] Login erfolgreich! Euer Session-Token: {}\x1b[0m", token);
                                }
                                TcpServerMessage::LoginFailed { reason } => {
                                    println!("\n\x1b[31m[System] Login fehlgeschlagen: {}\x1b[0m", reason);
                                }
                                TcpServerMessage::ReloginOk { token } => {
                                    println!("\n\x1b[32m[System] Relogin erfolgreich! Session {} aktiv.\x1b[0m", token);
                                }
                                TcpServerMessage::ReloginFailed { reason } => {
                                    println!("\n\x1b[31m[System] Relogin fehlgeschlagen: {}\x1b[0m", reason);
                                }
                                TcpServerMessage::LogoutOk => {
                                    println!("\n\x1b[34m[System] Erfolgreich abgemeldet.\x1b[0m");
                                }
                                TcpServerMessage::Kicked { reason } => {
                                    println!("\n\x1b[31m[System] Du wurdest gekickt: {}\x1b[0m", reason);
                                    std::process::exit(0); // Client beenden
                                }
                                TcpServerMessage::CommandOutput{answer: result, ..} => {
                                    println!("[{}] {}", result.0, result.1);
                                }
                                _ => {}
                            }

                            // Zeigt den Prompt danach wieder an
                            print!("> ");
                            io::stdout().flush().unwrap();
                        }
                        Err(e) => {
                            println!("\n[System] Fehler beim Deserialisieren der Serverantwort: {}", e);
                        }
                    }
                }
                Err(e) => {
                    println!("\n[System] Fehler beim Lesen vom Server-Socket: {}", e);
                    std::process::exit(1);
                }
            }
        }
    });


    // ---------------------------------------------------------
    // EINGABE-SCHLEIFE (Haupt-Thread)
    // ---------------------------------------------------------
    loop {
        print!("> ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        let msg = match input.trim() {
            "1" => TcpClientMessage::Login {
                password: "".into(), // Euer Passwort hier
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
                println!("\x1b[31m[System] SIMULIERE KABELBRUCH! Lege mich schlafen...\x1b[0m");
                // Wir ignorieren alles und frieren den Haupt-Thread ein.
                // Der Server denkt, wir sind noch da, aber wir reagieren auf nichts mehr.
                thread::sleep(std::time::Duration::from_secs(3600));
                break;
            }

            // Der neue Relogin-Knopf
            "11" => {
                print!("Bitte Session-Token eingeben: ");
                io::stdout().flush().unwrap();

                let mut token_str = String::new();
                io::stdin().read_line(&mut token_str).unwrap();

                match token_str.trim().parse::<u64>() {
                    Ok(token) => {
                        // Wir bauen die Relogin-Nachricht und schicken sie ab
                        TcpClientMessage::Relogin {
                            user_id: token,
                            clear_subscriptions: false,
                        }
                    }
                    Err(_) => {
                        println!("\x1b[31m[System] Ungültiges Token. Bitte eine Zahl eingeben.\x1b[0m");
                        continue; // Bricht diesen Durchlauf ab und zeigt wieder den "> " Prompt
                    }
                }
            }

            "0" => break, // Beendet die Schleife und damit das Programm

            command => TcpClientMessage::ExecuteCommand {command: command.into(), response_id: 0},
        };

        // Nachricht senden, Thread blockiert hier NICHT mehr auf eine Antwort!
        if let Ok(_bytes) = bincode::deserialize::<Vec<u8>>(&bincode::serialize(&msg).unwrap()) {
            // Nur eine kleine Sicherheit gegen kaputte Serialize-Aufrufe.
            // Besser direkt:
        }

        let bytes = bincode::serialize(&msg).unwrap();
        if write_stream.write_all(&bytes).is_err() {
            println!("[System] Konnte nicht senden. Verbindung tot?");
            break;
        }
    }
}