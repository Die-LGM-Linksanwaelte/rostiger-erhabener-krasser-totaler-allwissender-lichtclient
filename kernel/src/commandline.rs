use common::fixture;
use common::fixture::ChannelError::{ChannelAlreadyInUse, ChannelOutOfRange, UniverseOutOfRange};
use common::fixture::FixtureError::{
    ChannelError, FixtureNameAlreadyInUse, FixtureTypeNameAlreadyInUse, InvalidFixture,
    InvalidFixtureType, InvalidPropertyType, MissingProperty, MultipleColorOutputTypes,
};
use common::fixture::{Fixture, FixtureType};
use std::collections::HashMap;
use std::str::SplitAsciiWhitespace;

/// Checks if the given string is a valid command and executes it.
/// See '../help.txt` for a list of available commands.
pub(crate) fn parse_command(line: String) {
    let mut line_iter = line.split_ascii_whitespace();
    //We want to check the arg count, we don't want the command counted
    let arg_count = line_iter.clone().count().saturating_sub(1);
    match line_iter.next() {
        Some("help") => {
            const HELP_TEXT: &str = include_str!("../help.txt");
            println!("{}", HELP_TEXT);
        }

        Some("new") if arg_count % 2 == 1 && arg_count > 1 => {
            new_fixture_type(line_iter);
        }

        Some("new") => {
            println!(
                "Error: \"new\"-Command needs a name for the new Fixture-Type, and then a list of properties with \
            their channels."
            );
        }

        Some("add") if arg_count == 4 => {
            new_fixture(line_iter);
        }

        Some("add") => {
            println!(
                "Error: \"add\" needs a name, a fixture-type, a start-channel and a universe as arguments"
            )
        }

        Some("set") if arg_count == 3 => {
            set_value(line_iter);
        }

        Some("set") => {
            println!("Error: \"set\" needs a fixture, a property, and a value as arguments")
        }

        Some("type") if arg_count == 1 => {
            get_type(line_iter);
        }

        Some("type") => {
            print!("Error: \"type\" needs a fixture as argument");
        }

        //Temporary Commands, when GUI is here, this should be deleted.

        Some("create_debug") => {
            let args = "rgb red 0 green 1 blue 2".split_ascii_whitespace();
            new_fixture_type(args);
            for i in 0..50 {
                let name = i.to_string();
                let start_channel = (i * 3).to_string();
                let args = format!("{} rgb 0 {}", name.clone(), start_channel.clone());
                let args = args.split_ascii_whitespace();
                new_fixture(args);
            }
        }

        Some("set_all") if arg_count == 2 => {
            let property_type = line_iter.next().unwrap().to_string();
            let value = line_iter.next().unwrap().to_string();

            for i in 0..50 {
                let name = i.to_string();
                let args = format!("{} {} {}", name, property_type.clone(), value.clone());
                let args = args.split_ascii_whitespace();
                set_value(args);
            }
        }

        Some("break") => {
            let _dmx_config = fixture::DMX_CONFIGURATION.read().unwrap();
            let _universes = fixture::calculate_dmx_values();
            println!("Add a breakpoint at this point in the code to check the datastructures");
        }

        Some("list_ports") => {
            let ports = interface::enttec_dmx_usb_pro::EnttecDmxPro::list_ports();

            ports.iter().for_each(|port| {
                println!("{}", port);
            })
        }
        
        Some("setup_enttec") => {
            interface::interfaces::setup_entec(line_iter.next().unwrap());
        }

        Some("exit") => {
            std::process::exit(0);
        }

        _ => {
            println!("Unknown command. Please enter help, to get a list of commands.");
        }
    }
}

fn new_fixture_type(mut args: SplitAsciiWhitespace) {
    let name = args.next().unwrap().to_string();
    let mut properties: HashMap<String, (u16, Option<u16>)> = HashMap::new();

    //*******************************************
    //***Parsing the input to the right Format***
    //*******************************************
    while let Some(property) = args.next() {
        //We can do that without throwing an Error, because args has an even number of elements at this point
        let channel = args.next().unwrap();
        if let Err(_) = channel.parse::<u16>() {
            eprintln!("Error: \"{channel}\" is not a valid channel-number");
            return;
        } else {
            let channel = channel.parse::<u16>().unwrap();

            if property.ends_with("_f") {
                //fine-channels

                // Cut off the _f
                let property = &property[..(property.len() - 2)];
                if let Some((_, opt)) = properties.get_mut(property) {
                    if opt.is_none() {
                        *opt = Some(channel);
                    } else {
                        eprintln!("{property} can only have one Fine-Channel");
                        return;
                    }
                } else {
                    eprintln!(
                        "{property} needs to define a normal Channel, before defining an fine-Channel"
                    );
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

    //***********************************************************
    //**Creating the fixture_type and handling possible Errors***
    //***********************************************************
    let fixture_type = FixtureType::new(name.clone(), properties);
    match fixture_type {
        Err(ChannelError(ChannelAlreadyInUse(channel_type))) => {
            eprintln!("Error: The channel {channel_type} overlaps with another channel.");
        }

        Err(ChannelError(ChannelOutOfRange)) => {
            eprintln!(
                "Error: A Channel is higher than the size of the Universe. This is not yet supported"
            );
        }

        Err(FixtureTypeNameAlreadyInUse(name)) => {
            eprintln!("Error: The Fixture type name {name} is already used.");
        }

        Err(InvalidPropertyType(property_type)) => {
            eprintln!("Error: \"{property_type}\" is not a valid PropertyType");
        }

        Err(MultipleColorOutputTypes(error_message)) => {
            eprintln!("{error_message}");
        }

        Err(_) => {
            eprintln!("Error: new_fixture_type() threw an Error it shouldn't");
            None::<Fixture>.unwrap();
            // Mir ist langweilig, deswegen crashe ich hier, auf die lustigste und verwirrendste Art. Hier muss auch
            // gecrashed werden, weil das nie passieren sollte
        }

        Ok(()) => {
            println!("{} created successfully", name);
        }
    }
}

fn new_fixture(mut args: SplitAsciiWhitespace) {
    let name = args.next().unwrap().to_string();
    let fixture_type_name = args.next().unwrap().to_string();
    let universe = args.next().unwrap().to_string();
    let channel = args.next().unwrap().to_string();

    //*******************************************
    //***Parsing the input to the right Format***
    //*******************************************
    if let Err(_) = channel.parse::<u16>() {
        eprintln!("Error: \"{fixture_type_name}\" is not a valid channel-number");
        return;
    }
    let channel = channel.parse::<u16>().unwrap();

    if let Err(_) = universe.parse::<usize>() {
        eprintln!("Error: \"{fixture_type_name}\" is not a valid universe-number");
        return;
    }
    let universe = universe.parse().unwrap();

    //******************************************************
    //**Creating the fixture and handling possible Errors***
    //******************************************************
    let fixture = Fixture::new(fixture_type_name, channel, universe, name.clone());
    match fixture {
        Err(ChannelError(ChannelOutOfRange)) => {
            eprintln!("Error: fixture overflowes out of this remaining universe");
        }

        Err(ChannelError(UniverseOutOfRange)) => {
            panic!(
                "Fatal Error: Fixture created in Universe that does not exist. Normally, the programm should \
        automatically create an universe, but somehow, this hasn't happened"
            );
        }

        Err(ChannelError(ChannelAlreadyInUse(overlapping_fixture))) => {
            eprintln!(
                "Error: At least one Channel of this fixture is overlapping with {}.\
            Fixture has not been created.",
                overlapping_fixture
            );
        }

        Err(InvalidFixtureType(fixture_type_name)) => {
            eprintln!("Error: There is no fixture-type named \"{fixture_type_name}\".");
        }

        Err(FixtureNameAlreadyInUse(name)) => {
            eprintln!("Error: The Fixture name {name} is already used.");
        }

        Err(_) => {
            eprintln!("Error: new_fixture_type() threw an Error it shouldn't");
            None::<Fixture>.unwrap();
            // Mir ist langweilig, deswegen crashe ich hier, auf die lustigste und verwirrendste Art. Hier muss auch
            // gecrashed werden, weil das nie passieren sollte, und ich hab all das einfach von new_fixture_type kopiert
        }

        Ok(_) => {
            println!("{} created successfully", name);
        }
    }
}

fn set_value(mut args: SplitAsciiWhitespace) {
    let fixture_name = args.next().unwrap().to_string();
    let property_name = args.next().unwrap().to_string();
    let value = args.next().unwrap().to_string();

    //*******************************************
    //***Parsing the input to the right Format***
    //*******************************************
    if let Err(_) = value.parse::<u16>() {
        eprintln!("Error: \"{value}\" is not a valid value.");
        return;
    }
    let value = value.parse::<u16>().unwrap();

    //****************************************************
    //**Changing the value and handling possible Errors***
    //****************************************************
    let result = Fixture::set(fixture_name.clone(), &*property_name, value);
    match result {
        Err(InvalidPropertyType(property_type)) => {
            eprintln!("Error: \"{property_type}\" is not a valid PropertyType");
        }

        Err(MissingProperty(_)) => {
            eprintln!("Error: \"{fixture_name}\" has no property \"{property_name}\"")
        }

        Err(InvalidFixture(name)) => {
            eprintln!("Error: \"{name}\" is not a valid Fixture");
        }

        Err(_) => {
            eprintln!("Error: new_fixture_type() threw an Error it shouldn't");
            None::<Fixture>.unwrap();
            // Mir ist langweilig, deswegen crashe ich hier, auf die lustigste und verwirrendste Art. Hier muss auch
            // gecrashed werden, weil das nie passieren sollte, und ich hab all das einfach schon wieder von
            // new_fixture_type kopiert
        }

        Ok(_) => {
            println!("Value changed successfully");
        }
    }
}

fn get_type(mut args: SplitAsciiWhitespace) {
    let fixture_name = args.next().unwrap().to_string();

    match Fixture::get_fixture_type_from_string(fixture_name.clone()) {
        Ok(fixture_type) => {
            println!("\"{fixture_name}\" is a fixture of the type \"{fixture_type}\"")
        }
        Err(InvalidFixture(fixture)) => eprintln!("Error: \"{fixture}\" is not a valid Fixture"),
        Err(_) => panic!("Error: get_fixture_type_from_string() threw an Error it shouldn't"),
    }
}
