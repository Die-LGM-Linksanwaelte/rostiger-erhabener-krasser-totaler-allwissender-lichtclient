mod cli;
mod networking;

use std::io::{self, Write};
use std::thread;
use std::time::Duration;
use common::r_log;
use common::logging::{Logger, TerminalSink, FileSink};
use common::logging::LogLevel::*;
use interface::interfaces::dmx_output_loop;
use crate::cli::command_parsing::run_command;

/// Spawns the ['dmx_output_loop']-thread and then starts the main REPL.
fn main() -> io::Result<()> {

    if cfg!(all(debug_assertions, not(test))) {
        thread::sleep(Duration::from_millis(1000));
    }

    Logger::global().add_sink(Box::new(TerminalSink {cli_prompt: Some("> ".into())}));
    Logger::global().add_sink(Box::new(FileSink::new("/tmp/kernel.log")));

    let _artnet_handle = thread::spawn(|| {
        dmx_output_loop().expect("\x1b[31martnet loop failed\x1b[0m");
    });


    networking::activate_socket(6767);


    loop {
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read line");

        let input = input.trim().to_string();

        let response = run_command(input);

        r_log!(response.0, "{}", response.1);
    }

    _artnet_handle.join().unwrap();
}
