use std::io::Read;
use std::net::{TcpListener, TcpStream};
use std::thread;

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