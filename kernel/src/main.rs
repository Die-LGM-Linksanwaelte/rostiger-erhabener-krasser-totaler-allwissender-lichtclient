mod cli;
mod networking;
mod fixture;

use std::io::{self, Write};
use std::{env, thread};
use std::time::Duration;
use common::r_log;
use common::logging::{FileSink, Logger, TerminalSink};
use common::logging::LogLevel::*;
use interface::interfaces::dmx_output_loop;
use crate::cli::command_parsing::run_command;

/// Spawns the ['dmx_output_loop']-thread and then starts the main REPL.
fn main() -> io::Result<()> {

    let port = get_arguments();

    if cfg!(all(debug_assertions, not(test))) {
        thread::sleep(Duration::from_millis(1000));
    }

    Logger::global().add_sink(Box::new(TerminalSink {cli_prompt: Some("> ".into())}));
    Logger::global().add_sink(Box::new(FileSink::new("/tmp/kernel.log")));

    ctrlc::set_handler(move || {
        r_log!(
            Warning,
            "Ctrl+C is disabled to prevent data loss. Please type 'exit' to shutdown safely."
        );
    }).expect("Error setting Ctrl-C handler");

    let interface_receiver = fixture::FixtureEngine::spawn().expect("Failed to spawn FixtureEngine");

    let _artnet_handle = thread::spawn(|| {
        dmx_output_loop(interface_receiver).expect("\x1b[31martnet loop failed\x1b[0m");
    });


    networking::activate_socket(port);


    loop {
        io::stdout().flush()?;

        let mut input = String::new();
        if let Err(e) = io::stdin().read_line(&mut input) {
            r_log!(UserError, "Terminal input stream contained invalid UTF-8 (e.g. from deleting special characters).\
             Discarding input. Error: {}", e);
            continue;
        }

        let input = input.trim().to_string();

        let response = run_command(true, input);

        r_log!(response.0, "{}", response.1);
    }

    _artnet_handle.join().unwrap();
}

fn get_arguments() -> u16 {
    // Default values:
    let mut port: u16 = 6767;

    let args: Vec<String> = env::args().collect();
    let mut iter = args.iter().skip(1);

    while let Some(arg) = iter.next() {
        match arg.as_ref() {
            "--port" => {
                if let Some(port_str) = iter.next() {
                    match port_str.parse::<u16>() {
                        Ok(p) => port = p,
                        Err(_) => {
                            println!("Invalid port number: {}", port_str);
                            thread::sleep(Duration::from_millis(50));
                            std::process::exit(1);
                        }
                    }
                }
            }

            _ => {
                println!("Invalid argument: {}", arg);
                thread::sleep(Duration::from_millis(50));
                std::process::exit(1);
            },
        }
    }

    port
}
