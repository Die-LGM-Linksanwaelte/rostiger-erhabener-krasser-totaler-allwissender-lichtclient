use std::io::{self, Write};

mod commandline;

fn main() {
    loop {
        print!("> ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).expect("Failed to read line");

        let input = input.trim().to_string();

        commandline::parse_command(input);
    }
    println!("Hello, world!");
}
