use std::io::{self, Write};
use Interface::interfaces::dmx_output_loop;

mod commandline;

fn main() -> io::Result<()> {

    let artnet_handle = std::thread::spawn(|| {
        dmx_output_loop().expect("artnet loop failed");
    });


    loop {
        print!("> ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input).expect("Failed to read line");

        let input = input.trim().to_string();

        commandline::parse_command(input);
    }

    artnet_handle.join().unwrap();
    println!("Hello, world!");
}
