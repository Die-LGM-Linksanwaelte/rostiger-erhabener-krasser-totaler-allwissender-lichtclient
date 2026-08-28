use std::collections::HashMap;
use std::str::SplitAsciiWhitespace;
use common::logging::LogLevel;
use common::logging::LogLevel::*;
use common::fixture::ChannelError::{FineDegreeTooHigh, FineDegreeExists, FineDegreeOutOfRange};
use common::fixture::FixtureError::InvalidPropertyType;
use common::fixture::{
    ChannelIndex, ChannelValue, ChannelParameter, PropertyType, MAX_FINE_DEGREES,
    FloatChannelValue
};
use common::fixture::color::ColorPropertyType;
use common::cli_actions::CliAction;
use crate::cli::cli_executing::execute_cli_action;

pub fn run_command(is_kernel: bool, command: String) -> (LogLevel, String) {
    match parse_cli_string(command) {
        Ok(command) => execute_cli_action(is_kernel, &command),
        Err(e) => (UserError, e.to_string())
    }
}

pub(crate) fn parse_cli_string(command_string: String) -> Result<CliAction,String> {
    let mut line_iter = command_string.split_ascii_whitespace();
    let arg_count = line_iter.clone().count().saturating_sub(1);

    match line_iter.next() {
        Some("help") => Ok(CliAction::Help),

        Some("new") if arg_count % 2 == 1 && arg_count > 1 => {
            parse_new_fixture_type(line_iter)
        }

        Some("new") => {
            Err("Error: \"new\"-Command needs a name for the new Fixture-Type, and then a list of properties with \
            their channels.".to_string())
        }

        Some("add") if arg_count == 3 => {
            parse_new_fixture(line_iter)
        }

        Some("add") => {
            Err("Error: \"add\" needs a name, a fixture-type and a start-channel (including a start-universe) as \
            arguments".to_string())
        }

        Some("move") if arg_count == 2 => {
            parse_move_fixture(line_iter)
        }

        Some("move") => {
            Err("Error: \"move\" needs a fixture and a start-channel (including a start-universe) as \
            arguments".to_string())
        }

        Some("remove") if arg_count == 1 => {
            parse_remove_fixture(line_iter)
        }

        Some("remove") => {
            Err("Error: \"remove\" needs a fixture as argument".to_string())
        }

        Some("set") if arg_count == 3 => {
            parse_set_value(line_iter)
        }

        Some("set") => {
            Err("Error: \"set\" needs a fixture, a property, and a value as arguments".to_string())
        }

        Some("type") if arg_count == 1 => {
            parse_get_type(line_iter)
        }

        Some("type") => {
            Err("Error: \"type\" needs a fixture as argument".to_string())
        }

        Some("exit") if arg_count <= 1 => {
            parse_exit(line_iter)
        },

        Some("exit") => {
            Err("Error: \"exit\" accepts at most one optional argument ('save' or 'discard').".to_string())
        },

        Some(command) => {
            let args = line_iter.collect::<Vec<_>>().join(" ");
            Ok(CliAction::OtherCommands {
                command: format!("{} {}",command,args)
            })
        }

        _ => Err("Unknown command. Please enter help, to get a list of commands.".to_string())
    }
}

/// Checks if the given string is a valid command and executes it.
/// See '../help.txt' for a list of available commands.
pub fn parse_debug_command(line: String) -> (LogLevel, String) {
    let mut line_iter = line.split_ascii_whitespace();
    //We want to check the arg count, we don't want the command counted
    let arg_count = line_iter.clone().count().saturating_sub(1);
    match line_iter.next() {

        Some("create_debug") if cfg!(all(debug_assertions, not(test))) => {
            let fixture_type_name = "rgb".to_string();
            let universe = 0;

            let mut channels = HashMap::new();
            channels.insert(PropertyType::Color(ColorPropertyType::Red), ChannelParameter::new(0));
            channels.insert(PropertyType::Color(ColorPropertyType::Green), ChannelParameter::new(1));
            channels.insert(PropertyType::Color(ColorPropertyType::Blue), ChannelParameter::new(2));

            let new_command = CliAction::FixtureNew {
                name: fixture_type_name.clone(),
                channels,
            };

            if let (UserError,error) = execute_cli_action(false,&new_command) {
                return (UserError,error.to_string());
            }
            for i in 0..50 {
                let name = i.to_string();
                let start_channel = i * 3;

                let add_command = CliAction::FixtureAdd {
                    name,
                    fixture_type_name: fixture_type_name.clone(),
                    universe,
                    channel: start_channel
                };

                match execute_cli_action(false, &add_command) {
                    (UserSuccess, _) => continue,
                    x => return x
                }
            }
            (UserSuccess,"Created the debug-fixtures".to_string())
        }

        Some("set_all") if arg_count == 2 && cfg!(all(debug_assertions, not(test))) => {
            let property_name = line_iter.next().unwrap().to_string();
            let value = line_iter.next().unwrap().to_string();

            let property_type = match PropertyType::from_str(&*property_name) {
                Ok(property_type) => property_type,
                Err(InvalidPropertyType(property_type)) => {
                    return (UserError,format!("Error: \"{property_type}\" is not a valid PropertyType"))
                }
                Err(_) => unreachable!() //All possible Errors have been handles
            };

            let value = match parse_cli_value(&*value) {
                Ok(value) => value,
                Err(e) => return (UserError,e.to_string()),
            };

            for i in 0..50 {
                let name = i.to_string();
                let set_command = CliAction::FixtureSet {
                    name,
                    property_type: property_type.clone(),
                    value,
                };

                match execute_cli_action(false, &set_command) {
                    (UserSuccess, _) => continue,
                    x => return x
                }
            }

            (UserSuccess,format!("Set {} to {} in all debug-fixtures", property_type, value))
        }

        Some("break") if cfg!(all(debug_assertions, not(test))) => {
            (Info,"Add a breakpoint at this point in the code to check the datastructures".to_string())
        }

        Some(command) => {
            (UserError,format!("Unknown command \"{command}\". Please enter help, to get a list of commands."))
        }

        None => (UserError,"Unknown command. Please enter help, to get a list of commands.".to_string()),
    }
}

fn parse_new_fixture_type(mut args: SplitAsciiWhitespace) -> Result<CliAction,String> {
    let name = args.next().unwrap().to_string();
    let mut properties: HashMap<PropertyType, ChannelParameter> = HashMap::new();

    while let Some(property_name) = args.next() {
        //We can do that without throwing an Error, because args has an even number of elements at this point
        let channel = args.next().unwrap();

        let channel_index = match channel.parse::<ChannelIndex>() {
            Ok(channel) => channel,
            Err(_) => {
                return Err("Error: \"{channel}\" is not a valid channel-number".to_string());
            }
        };

        //fine-channels
        if let Some((property_name, suffix)) = property_name.rsplit_once("_f") {

            let fine_degree = if suffix.is_empty() {
                1
            } else {
                match suffix.parse::<u8>() {
                    Ok(fine_degree) => fine_degree,
                    Err(_) => return Err(format!("Error: \"{suffix}\" is not a valid fine degree"))
                }
            };

            let property_type = parse_property_type(property_name)?;

            match properties.get_mut(&property_type) {
                Some(channel_object) => {
                    match channel_object.add_fine(fine_degree.into(), channel_index) {
                        Err(FineDegreeTooHigh(f)) =>
                            return Err(format!("Fine-degree {} is too high, add the lower ones first", f)),
                        Err(FineDegreeExists(f)) =>
                            return Err(format!("Fine-degree {} already exists", f)),
                        Err(FineDegreeOutOfRange(f)) =>
                            return Err(format!("Fine-degree {} out of range, must be lower than {}",
                                               f, MAX_FINE_DEGREES)),
                        Ok(()) => {}
                        _ => unreachable!()
                    }
                }

                None => return Err(format!(
                    "Error: Cannot add fine channel for '{}' because the coarse channel is missing! Add '{}' first.",
                    property_name, property_type)),
            }
        } else {
            //Non-fine channels

            let property_type = parse_property_type(property_name)?;

            if !properties.contains_key(&property_type) {
                properties.insert(property_type, ChannelParameter::new(channel_index));
            } else {
                return Err(format!("{property_name} can only have one coarse Channel"))
            }
        }
    }
    Ok(CliAction::FixtureNew {
        name,
        channels: properties,
    })

}

fn parse_new_fixture(mut args: SplitAsciiWhitespace) -> Result<CliAction,String> {
    let name = args.next().unwrap().to_string();
    let fixture_type_name = args.next().unwrap().to_string();
    let (universe, channel) = parse_universe_and_channel(args)?;

    Ok(CliAction::FixtureAdd {
        name,
        fixture_type_name,
        channel,
        universe,
    })
}


fn parse_move_fixture(mut args: SplitAsciiWhitespace) -> Result<CliAction, String> {
    let fixture_name = args.next().unwrap().to_string();
    let (new_universe, new_channel) = parse_universe_and_channel(args)?;

    Ok(CliAction::FixtureMove {
        fixture_name,
        new_channel,
        new_universe
    })
}

fn parse_remove_fixture(mut args: SplitAsciiWhitespace) -> Result<CliAction, String> {
    let fixture_name = args.next().unwrap().to_string();

    Ok(CliAction::FixtureRemove {
        fixture_name
    })
}

fn parse_set_value(mut args: SplitAsciiWhitespace) -> Result<CliAction,String> {
    let name = args.next().unwrap().to_string();
    let property_name = args.next().unwrap().to_string();
    let value = args.next().unwrap().to_string();

    let property_type = match PropertyType::from_str(&property_name) {
        Ok(property_type) => property_type,
        Err(InvalidPropertyType(property_type)) => {
            return Err(format!("Error: \"{property_type}\" is not a valid PropertyType"))
        }
        Err(_) => unreachable!() //All possible Errors have been handles
    };

    let value = parse_cli_value(&*value)?;

    Ok(CliAction::FixtureSet {
        name,
        property_type,
        value,
    })
}

fn parse_get_type(mut args: SplitAsciiWhitespace) -> Result<CliAction,String> {
    let fixture_name = args.next().unwrap().to_string();

    Ok(CliAction::FixtureGetType {
        fixture_name,
    })
}

fn parse_cli_value(input: &str) -> Result<ChannelValue,String> {

    let sanitized_input = input.trim().replace("_", "");
    let input_str = sanitized_input.as_str();

    if let Some(percent_string) = input_str.strip_suffix("%") {
        match percent_string.parse::<FloatChannelValue>() {
            Ok(p) if (0.0..=100.0).contains(&p) => {
                let fraction = p / 100.0;
                let raw_value = fraction * (ChannelValue::MAX as FloatChannelValue);

                Ok(raw_value.round() as ChannelValue)
            }
            Ok(_) => Err(format!("\"{}\" must be between 0 and 100.", percent_string)),
            Err(_) => Err(format!("Invalid percantage format {}", percent_string)),
        }
    } else if let Some(hex_str) = input_str.strip_prefix("#") {
        let hex_len = hex_str.len();

        match ChannelValue::from_str_radix(hex_str, 16) {
            Ok(val) => {
                let scaled_val = match hex_len {
                    2 => val * 0x01010101,
                    4 => val * 0x00010001,
                    6 => val << 8,
                    8 => val,
                    _ => return Err(format!(
                        "Unsupported hex length: {}. Valid lengths are 2, 4, 6, or 8 digits (excluding '_').",
                        hex_len
                    )),
                };
                Ok(scaled_val)
            }

            Err(_) => Err(format!("Invalid hex format {}", hex_str))
        }
    } else {
        input_str.parse::<ChannelValue>()
            .map_err(|_| format!("Invalid value {}. ", input_str))
    }
}


fn parse_exit(mut args: SplitAsciiWhitespace) -> Result<CliAction,String> {
    match args.next() {
        Some("save") => Ok(CliAction::Exit { save_changes: Some(true) }),
        Some("discard") => Ok(CliAction::Exit { save_changes: Some(false) }),
        Some(invalid) => Err(format!("Error: Invalid argument '{}' for exit. Use 'save' or 'discard'.", invalid)),
        None => Ok(CliAction::Exit { save_changes: None }),
    }
}


fn parse_universe_and_channel(mut args: SplitAsciiWhitespace) -> Result<(usize, ChannelIndex),String> {
    let parsed_string = args.next().unwrap();
    let (universe, channel) = match parsed_string.split_once(".") {
        Some(pair) => pair,
        None => return Err("Error: Please specify the channel with [universe].[channel]".to_string()),
    };
    let universe = match universe.parse::<usize>() {
        Ok(universe) => universe,
        Err(_) => {
            return Err(format!("Error: \"{universe}\" is not a valid universe-number"))
        }
    };

    let channel = match channel.parse::<ChannelIndex>() {
        Ok(channel) => channel,
        Err(_) => {
            return Err(format!("Error: \"{}\" is not a valid channel-number", channel))
        }
    };
    Ok((universe, channel))
}



fn parse_property_type(property_name: &str) -> Result<PropertyType, String> {
    Ok(match PropertyType::from_str(property_name) {
        Ok(property_type) => property_type,
        Err(InvalidPropertyType(property_type)) => {
            return Err(format!("Error: \"{property_type}\" is not a valid PropertyType"))
        }
        Err(_) => unreachable!() //All possible Errors have been handles
    })
}