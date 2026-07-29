use interface::interfaces::dmx_output_loop;
use std::io::{self, Write};
use std::thread;
use std::time::Duration;
use common::logging::{Logger, TerminalSink, FileSink};
use common::logging::LogLevel::*;
use common::r_log;
use crate::commandline::parse_command;

mod commandline;

/// Spawns the ['dmx_output_loop']-thread and than starts the main REPL.
fn main() -> io::Result<()> {

    if cfg!(all(debug_assertions, not(test))) {
        thread::sleep(Duration::from_millis(1000));
    }

    Logger::global().add_sink(Box::new(TerminalSink {cli_prompt: Some("> ".into())}));
    Logger::global().add_sink(Box::new(FileSink::new("kernel.log")));

    let _artnet_handle = std::thread::spawn(|| {
        dmx_output_loop().expect("\x1b[31martnet loop failed\x1b[0m");
    });


    common::networking::server_sockets::activate_socket(6767, |cmd| {
        parse_command(cmd)
    });


    loop {
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read line");

        let input = input.trim().to_string();

        match parse_command(input) {
            Ok(output) => {
                r_log!(UserSuccess, "{}", output);
            },

            Err(output) => {
                r_log!(UserError,"{}", output);
            }
        }
    }

    _artnet_handle.join().unwrap();
}
