use std::collections::HashMap;
use std::str::SplitAsciiWhitespace;
use FixtureTest::fixture;
use FixtureTest::fixture::FixtureType;
use FixtureTest::fixture::ParseError::InvalidPropertyType;

pub(crate) fn parse_command(line: String) {

    let mut line_iter = line.split_ascii_whitespace();
    //We want to check the arg count, we don't want the command counted
    let arg_count = line_iter.clone().count() - 1;
    match line_iter.next() {

        Some("help") => {
            const HELP_TEXT: &str = include_str!("../help.txt");
            println!("{}", HELP_TEXT);
        }

        Some("new") if arg_count % 2 == 1 && arg_count > 1 => {
            new_fixture_type(line_iter);
        }

        Some("new") => {
            println!("Error: \"new\"-Command needs a name for the new Fixture-Type, and then a list of properties with \
            their channels.");
        }

        Some("add") => {

        }

        Some("set") => {

        }

        Some("type") => {

        }

        _ => {
            println!("Unknown command. Please enter help, to get a list of commands.");
        }
    }
}

fn new_fixture_type(mut args: SplitAsciiWhitespace) {
    let name = args.next().unwrap().to_string();
    let mut properties: HashMap<String, (u16, Option<u16>)> = HashMap::new();
    while let Some(property) = args.next() {
        //We can do that without throwing an Error, because args has an even number of elements at this point
        let channel = args.next().unwrap();
        if let Err(_) = channel.parse::<u16>() {
            eprintln!("Error: \"{channel}\" is not a valid channel-number");
            return;
        } else {
            let channel = channel.parse::<u16>().unwrap();

            if channel > 511 {
                eprintln!("Error: \"{channel}\" is out of range");
                return;
            }

            if property.ends_with("_f") {
                //fine-channels

                // Cut off the _f
                let property =  &property[..(property.len() - 2)];
                if let Some((_, opt)) = properties.get_mut(property) {
                    if opt.is_none() {
                        *opt = Some(channel);
                    } else {
                        eprintln!("{property} can only have one Fine-Channel");
                        return;
                    }
                } else {
                    eprintln!("{property} needs to define a normal Channel, before defining an fine-Channel");
                    return;
                }
            } else {
                //Non-fine channels

                if !properties.contains_key(property) {
                    properties.insert(property.to_string(), (channel, None));
                } else {
                    eprintln!("{property} can only have one non-fine Channel");
                    return;
                }
            }
        }

    }
    let mut list = fixture::FIXTURE_LIST.lock().unwrap();
    let fixture_type = FixtureType::new(name.clone(), properties);
    if let Ok(fixture_type) = fixture_type {
        list.fixture_types.insert(name, fixture_type);
    } else if let Err(fixture_type) = fixture_type {
        let InvalidPropertyType(fixture_type) = fixture_type;
        eprintln!("Error: \"{fixture_type}\" is not a valid FixtureType");
    }
}
