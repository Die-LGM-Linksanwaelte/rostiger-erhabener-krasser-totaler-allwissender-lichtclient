mod fixture;
mod networking;

use std::net::TcpStream;
use networking::messages::{TcpClientMessage, UpdateMode, SubscribeTopic, UserRole};

///startPoint - This is the main entry point of the common application.
fn main() {
    println!("EDER stinkt!");
    use std::io::{self, Write};

    let mut stream = TcpStream::connect("127.0.0.1:6767")
        .expect("Verbindung fehlgeschlagen");

    loop {
        println!();
        println!("1 Connect");
        println!("2 Subscribe Universes OnChange");
        println!("3 Subscribe Universes Continuous");
        println!("4 Subscribe FixturePositions OnChange");
        println!("5 Subscribe FixturePositions Continuous");
        println!("6 Unsubscribe Universes");
        println!("7 Unsubscribe FixturePositions");
        println!("8 Disconnect");
        println!("0 Exit");

        print!("> ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        let msg = match input.trim() {
            "1" => TcpClientMessage::Connect {
                user_id: 42,
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

            _ => continue,
        };

        let bytes = bincode::serialize(&msg).unwrap();
        stream.write_all(&bytes).unwrap();
    }
}