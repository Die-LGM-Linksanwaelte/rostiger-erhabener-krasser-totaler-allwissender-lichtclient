use std::collections::HashMap;
use std::str::SplitAsciiWhitespace;
use FixtureTest::fixture;
use FixtureTest::fixture::{Fixture, FixtureType};
use FixtureTest::fixture::ParseError::{InvalidPropertyType, MultipleColorOutputTypes};

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

        Some("add") if arg_count == 3 => {
            new_fixture(line_iter);
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
        match list.fixture_types.entry(name.clone()) {
            std::collections::hash_map::Entry::Occupied(_) => {
                eprintln!("Error: \"{name}\" already exists");
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(fixture_type);
            }
        }
    } else if let Err(fixture_type) = fixture_type {
        if let InvalidPropertyType(fixture_type) = fixture_type {
            eprintln!("Error: \"{fixture_type}\" is not a valid FixtureType");
        } else if let MultipleColorOutputTypes(error_message) = fixture_type {
            eprintln!("{error_message}");
        } else { 
            unreachable!();
        }
        return;
    }
}

fn new_fixture(mut args: SplitAsciiWhitespace) {
    let name = args.next().unwrap().to_string();
    let fixture_type_name = args.next().unwrap().to_string();
    let channel = args.next().unwrap().to_string();
    
    if let Err(_) = channel.parse::<u16>() {
        eprintln!("Error: \"{fixture_type_name}\" is not a valid channel-number");
        return;
    }
    let channel = channel.parse::<u16>().unwrap();

    if channel > 511 {
        eprintln!("Error: \"{channel}\" is out of range");
        return;
    }
    
    let mut list = fixture::FIXTURE_LIST.lock().unwrap();
    if let None = list.fixture_types.get(&fixture_type_name) {
        eprintln!("Error: \"{fixture_type_name}\" is not a valid FixtureType");
        return;
    }
    let fixture_type= list.fixture_types.get(&fixture_type_name).unwrap();

    let fixture = Fixture::new(
        fixture_type,channel,name.clone()
    );

    if let Err(_) = fixture {
        eprintln!("Error: fixture is too big to fit into this remaining universe");
        return;
    }

    match list.fixtures.entry(name.clone()) {
        std::collections::hash_map::Entry::Occupied(_) => {
            eprintln!("Error: \"{name}\" already exists");
        }
        std::collections::hash_map::Entry::Vacant(entry) => {
            entry.insert(fixture.unwrap());
        }
    }
    
}
