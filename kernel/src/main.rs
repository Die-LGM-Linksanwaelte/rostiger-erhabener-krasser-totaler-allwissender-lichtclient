use crate::commandline::parse_command;
use interface::interfaces::dmx_output_loop;
use std::io::{self, Write};
use std::thread;
use std::time::Duration;

mod commandline;

/// Spawns the ['dmx_output_loop']-thread an than starts the main REPL.
fn main() -> io::Result<()> {
    let artnet_handle = std::thread::spawn(|| {
        dmx_output_loop().expect("\x1b[31martnet loop failed\x1b[0m");
    });

    let network_handle = std::thread::spawn(|| {
        common::networking::server_sockets::activate_socket(6767, |cmd| parse_command(cmd));
    });

    thread::sleep(Duration::new(0, 1_000_000));
    loop {
        print!("> ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read line");

        let input = input.trim().to_string();

        match parse_command(input) {
            Ok(output) => {
                println!("\x1b[32m{}\x1b[0m", output);
            }

            Err(output) => {
                eprintln!("\x1b[33m{}\x1b[0m", output);
            }
        }
    }

    artnet_handle.join().unwrap();
    println!("Hello, world!");
}
