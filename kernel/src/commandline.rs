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
/// See '../help.txt' for a list of available commands.
pub(crate) fn parse_command(line: String) -> Result<String,String> {
    let mut line_iter = line.split_ascii_whitespace();
    //We want to check the arg count, we don't want the command counted
    let arg_count = line_iter.clone().count().saturating_sub(1);
    match line_iter.next() {
        Some("help") => {
            const HELP_TEXT: &str = include_str!("../help.txt");
            Ok(HELP_TEXT.to_string())
        }

        Some("new") if arg_count % 2 == 1 && arg_count > 1 => {
            new_fixture_type(line_iter)
        }

        Some("new") => {
            Err("Error: \"new\"-Command needs a name for the new Fixture-Type, and then a list of properties with \
            their channels.".to_string())
        }

        Some("add") if arg_count == 4 => {
            new_fixture(line_iter)
        }

        Some("add") => {
            Err("Error: \"add\" needs a name, a fixture-type, a start-channel and a universe as arguments".to_string())
        }

        Some("set") if arg_count == 3 => {
            set_value(line_iter)
        }

        Some("set") => {
            Err("Error: \"set\" needs a fixture, a property, and a value as arguments".to_string())
        }

        Some("type") if arg_count == 1 => {
            get_type(line_iter)
        }

        Some("type") => {
            Err("Error: \"type\" needs a fixture as argument".to_string())
        }

        Some("create_debug") => {
            let args = "rgb red 0 green 1 blue 2".split_ascii_whitespace();
            if let Err(error) = new_fixture_type(args) {
                return Err(error.to_string());
            }
            for i in 0..50 {
                let name = i.to_string();
                let start_channel = (i * 3).to_string();
                let args = format!("{} rgb 0 {}", name.clone(), start_channel.clone());
                let args = args.split_ascii_whitespace();
                if let Err(error) = new_fixture(args) {
                    return Err(error.to_string());
                }
            }
            Ok("Created the debug-fixtures".to_string())
        }

        Some("set_all") if arg_count == 2 => {
            let property_type = line_iter.next().unwrap().to_string();
            let value = line_iter.next().unwrap().to_string();

            for i in 0..50 {
                let name = i.to_string();
                let args = format!("{} {} {}", name, property_type.clone(), value.clone());
                let args = args.split_ascii_whitespace();
                if let Err(error) = set_value(args) {
                    return Err(error);
                }
            }

            Ok(format!("Set {property_type} to {value} in all debug-fixtures"))
        }

        Some("break") => {
            let _dmx_config = fixture::DMX_CONFIGURATION.read().unwrap();
            let _universes = fixture::calculate_dmx_values();
            Ok("Add a breakpoint at this point in the code to check the datastructures".to_string())
        }

        _ => {
            Err("Unknown command. Please enter help, to get a list of commands.".to_string())
        }
    }
}

fn new_fixture_type(mut args: SplitAsciiWhitespace) -> Result<String,String> {
    let name = args.next().unwrap().to_string();
    let mut properties: HashMap<String, (u16, Option<u16>)> = HashMap::new();

    //*******************************************
    //***Parsing the input to the right Format***
    //*******************************************
    while let Some(property) = args.next() {
        //We can do that without throwing an Error, because args has an even number of elements at this point
        let channel = args.next().unwrap();
        if let Err(_) = channel.parse::<u16>() {
            return Err("Error: \"{channel}\" is not a valid channel-number".to_string());
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
                        return Err(format!("{property} can only have one Fine-Channel"));
                    }
                } else {
                    return Err(format!("{property} needs to define a normal Channel, before defining an fine-Channel"));
                }
            } else {
                //Non-fine channels

                if !properties.contains_key(property) {
                    properties.insert(property.to_string(), (channel, None));
                } else {
                    return Err(format!("{property} can only have one non-fine Channel"))
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
            Err(format!("Error: The channel {channel_type} overlaps with another channel."))
        }

        Err(ChannelError(ChannelOutOfRange)) => {
            Err("Error: A Channel is higher than the size of the Universe. This is not yet supported".to_string())
        }

        Err(FixtureTypeNameAlreadyInUse(name)) => {
            Err(format!("Error: The Fixture type name {name} is already used."))
        }

        Err(InvalidPropertyType(property_type)) => {
            Err(format!("Error: \"{property_type}\" is not a valid PropertyType"))
        }

        Err(MultipleColorOutputTypes(error_message)) => {
            Err(error_message)
        }

        Err(_) => {
            eprintln!("Error: new_fixture_type() threw an Error it shouldn't");
            None::<Fixture>.unwrap();
            unreachable!();
            // Mir ist langweilig, deswegen crashe ich hier, auf die lustigste und verwirrendste Art. Hier muss auch
            // gecrashed werden, weil das nie passieren sollte
        }

        Ok(()) => {
            Ok(format!("{} created successfully", name))
        }
    }
}

fn new_fixture(mut args: SplitAsciiWhitespace) -> Result<String,String> {
    let name = args.next().unwrap().to_string();
    let fixture_type_name = args.next().unwrap().to_string();
    let universe = args.next().unwrap().to_string();
    let channel = args.next().unwrap().to_string();

    //*******************************************
    //***Parsing the input to the right Format***
    //*******************************************
    if let Err(_) = channel.parse::<u16>() {
        return Err(format!("Error: \"{fixture_type_name}\" is not a valid channel-number"))
    }
    let channel = channel.parse::<u16>().unwrap();

    if let Err(_) = universe.parse::<usize>() {
        return Err(format!("Error: \"{fixture_type_name}\" is not a valid universe-number"))
    }
    let universe = universe.parse().unwrap();

    //******************************************************
    //**Creating the fixture and handling possible Errors***
    //******************************************************
    let fixture = Fixture::new(fixture_type_name, channel, universe, name.clone());
    match fixture {
        Err(ChannelError(ChannelOutOfRange)) => {
            Err("Error: fixture overflows out of this remaining universe".to_string())
        }

        Err(ChannelError(UniverseOutOfRange)) => {
            panic!(
                "Fatal Error: Fixture created in Universe that does not exist. Normally, the programm should \
        automatically create an universe, but somehow, this hasn't happened"
            );
        }

        Err(ChannelError(ChannelAlreadyInUse(overlapping_fixture))) => {
            Err(format!(
                "Error: At least one Channel of this fixture is overlapping with {}. Fixture has not been created.",
                overlapping_fixture
            ))
        }

        Err(InvalidFixtureType(fixture_type_name)) => {
            Err(format!("Error: There is no fixture-type named \"{fixture_type_name}\"."))
        }

        Err(FixtureNameAlreadyInUse(name)) => {
            Err(format!("Error: The Fixture name {name} is already used."))
        }

        Err(_) => {
            eprintln!("Error: new_fixture_type() threw an Error it shouldn't");
            None::<Fixture>.unwrap();
            unreachable!()
            // Mir ist langweilig, deswegen crashe ich hier, auf die lustigste und verwirrendste Art. Hier muss auch
            // gecrashed werden, weil das nie passieren sollte, und ich hab all das einfach von new_fixture_type kopiert
        }

        Ok(_) => {
            Ok(format!("{} created successfully", name))
        }
    }
}

fn set_value(mut args: SplitAsciiWhitespace) -> Result<String,String> {
    let fixture_name = args.next().unwrap().to_string();
    let property_name = args.next().unwrap().to_string();
    let value = args.next().unwrap().to_string();

    //*******************************************
    //***Parsing the input to the right Format***
    //*******************************************
    if let Err(_) = value.parse::<u16>() {
        return Err(format!("Error: \"{value}\" is not a valid value."))
    }
    let value = value.parse::<u16>().unwrap();

    //****************************************************
    //**Changing the value and handling possible Errors***
    //****************************************************
    let result = Fixture::set(fixture_name.clone(), &*property_name, value);
    match result {
        Err(InvalidPropertyType(property_type)) => {
            Err(format!("Error: \"{property_type}\" is not a valid PropertyType"))
        }

        Err(MissingProperty(_)) => {
            Err(format!("Error: \"{fixture_name}\" has no property \"{property_name}\""))
        }

        Err(InvalidFixture(name)) => {
            Err(format!("Error: \"{name}\" is not a valid Fixture"))
        }

        Err(_) => {
            eprintln!("Error: new_fixture_type() threw an Error it shouldn't");
            None::<Fixture>.unwrap();
            unreachable!()
            // Mir ist langweilig, deswegen crashe ich hier, auf die lustigste und verwirrendste Art. Hier muss auch
            // gecrashed werden, weil das nie passieren sollte, und ich hab all das einfach schon wieder von
            // new_fixture_type kopiert
        }

        Ok(_) => {
            Ok(format!("Value {property_name} of {fixture_name} changed successfully to {value}"))
        }
    }
}

fn get_type(mut args: SplitAsciiWhitespace) -> Result<String,String> {
    let fixture_name = args.next().unwrap().to_string();

    match Fixture::get_fixture_type_from_string(fixture_name.clone()) {
        Ok(fixture_type) => Ok(format!("\"{fixture_name}\" is a fixture of the type \"{fixture_type}\"")),
        Err(InvalidFixture(fixture)) => Ok(format!("Error: \"{fixture}\" is not a valid Fixture")),
        Err(_) => panic!("Error: get_fixture_type_from_string() threw an Error it shouldn't"),
    }
}
