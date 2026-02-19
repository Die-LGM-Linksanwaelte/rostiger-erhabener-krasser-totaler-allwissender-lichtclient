pub(crate) fn parse_command(line: String) {
    let mut line_iter = line.split_ascii_whitespace();
    match line_iter.next() {

        Some("help") => {
            const HELP_TEXT: &str = include_str!("../help.txt");
            println!("{}", HELP_TEXT);
        }

        Some("new") => {
            
        }

        Some("add") => {

        }

        Some("set") => {

        }

        _ => {
            println!("Unknown command. plese enter help, to get a list of commands.");
        }
    }
}
