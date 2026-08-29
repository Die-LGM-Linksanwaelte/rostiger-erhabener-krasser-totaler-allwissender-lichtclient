//! # Rektal Lighting Control Kernel
//!
//! This is the main entry point for the Rektal lighting control kernel executable.
//! It initializes the core logging infrastructure, parses command-line arguments,
//! boots the fixture engine and DMX output interfaces, activates the TCP networking layer
//! for client communication, and runs the primary interactive REPL shell.
mod cli;
mod networking;
mod fixture;

use std::io::{self, Write};
use std::{env, thread};
use std::time::Duration;
use common::{r_debug_log, r_log};
use common::logging::{FileSink, Logger, TerminalSink};
use common::logging::LogLevel::*;
use interface::interfaces::dmx_output_loop;
use crate::cli::run_command;

/// Spawns the background fixture and DMX output worker threads, activates the TCP network socket,
/// and enters the main REPL (Read-Eval-Print Loop) to process interactive user commands.
///
/// # Errors
///
/// Returns an `io::Error` if terminal input reading or output flushing fails.
fn main() -> io::Result<()> {

    let port = get_arguments();

    if cfg!(all(debug_assertions, not(test))) {
        thread::sleep(Duration::from_millis(1000));
    }

    Logger::global().add_sink(Box::new(TerminalSink {cli_prompt: Some("> ".into())}));
    Logger::global().add_sink(Box::new(FileSink::new("kernel.log")));

    #[cfg(all(not(debug_assertions), not(test)))]
    {
        ctrlc::set_handler(move || {
            r_log!(
            Warning,
            "Ctrl+C is disabled to prevent data loss. Please type 'exit' to shutdown safely."
        );
        })
            .expect("Error setting Ctrl-C handler");
    }


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

        r_debug_log!(Info, "[Kernel Cli] User input: {}", input);


        let response = run_command(true, input);

        r_log!(response.0, "[Kernel Cli] {}", response.1);
    }
}

/// Parses command-line arguments passed to the kernel executable.
///
/// Supports the `--port [port]` argument to override the default TCP listening port (`6767`).
/// If an invalid port number or unknown argument is encountered, an error message is printed
/// and the process exits with a non-zero status code.
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
