use crate::debug_panic_or_return_log;
use std::collections::HashMap;
use std::io;
use std::io::Write;
use crate::r_log;
use crate::networking::announce_shutdown;
use common::cli_actions::{CliAction, CliActionResponse};
use common::cli_actions::CliActionResponse::{Ack, FixtureError, FixtureTypeInfo, UnsupportedCommand};
use common::logging::LogLevel;
use common::logging::LogLevel::{Info, UserError, UserSuccess, Warning};
use common::fixture::{ChannelIndex, ChannelValue, ChannelParameter, PropertyType, FixtureType, ColorPropertyType};
use common::fixture::ChannelError::{ChannelAlreadyInUse, ChannelOutOfRange, UniverseOutOfRange};
use common::fixture::FixtureError::{ChannelError, DmxStateDesync, FixtureNameAlreadyInUse, FixtureTypeNameAlreadyInUse, InvalidFixture, InvalidFixtureType, InvalidPropertyType, MissingProperty, MultipleColorOutputTypes};
use common::r_debug_log;
use crate::fixture;

/// Executes a structured [`CliAction`], returning a tuple containing the resulting [`LogLevel`] and a message string.
///
/// # Arguments
///
/// * `is_kernel`   - Flag indicating whether the action is executed from the kernel console
/// * `cli_action`  - Reference to the [`CliAction`] to execute
pub(super) fn execute_cli_action(is_kernel: bool, cli_action: &CliAction) -> (LogLevel, String) {
    match cli_action {
        CliAction::Help => {
            const HELP_TEXT: &str = include_str!("../../../common/help.txt");
            (UserSuccess, HELP_TEXT.to_string())
        },

        CliAction::FixtureNew { name, channels } => {
            new_fixture_type(name.clone(), channels.clone())
        },

        CliAction::FixtureAdd { name, fixture_type_name, universe, channel } => {
            new_fixture(name.clone(), fixture_type_name.clone(), *universe, *channel)
        },
        
        CliAction::FixtureMove {fixture_name, new_universe, new_channel} => {
            move_fixture(fixture_name.clone(), *new_universe, *new_channel)
        }
        
        CliAction::FixtureRemove { fixture_name } => {
            remove_fixture(fixture_name.clone())
        }

        CliAction::FixtureSet {name, property_type, value} => {
            set_property_value(name.clone(), property_type.clone(), *value)
        },

        CliAction::FixtureGetType { fixture_name } => {
            get_fixture_type(fixture_name.clone())
        },

        CliAction::Exit {save_changes} => {
            if !is_kernel {
                (UserError, "Only kernel can exit a session".to_string())
            } else {
                shutdown_kernel(save_changes);
                (Info, "Shutdown aborted".to_string())
                //If the programm exits the previous methode, the shutdown is aborted.
            }

        },

        CliAction::OtherCommands {command} => {
            execute_debug_command(command.clone())
        },

        //_ => (Error, "Not yet implemented".to_string())
    }
}

/// Checks if the given string is a valid debug command and executes it.
/// Returns a tuple containing the resulting [`LogLevel`] and a message string.
///
/// # Arguments
///
/// * `line` - The raw debug command string
fn execute_debug_command(line: String) -> (LogLevel, String) {
    let mut line_iter = line.split_ascii_whitespace();
    //We want to check the arg count, we don't want the command counted
    let arg_count = line_iter.clone().count().saturating_sub(1);
    match line_iter.next() {

        Some("create_debug") if cfg!(all(debug_assertions, not(test))) => {
            let fixture_type_name = "rgb".to_string();
            let universe = 0;

            let mut channels = HashMap::new();
            channels.insert(
                PropertyType::Color(ColorPropertyType::Red), ChannelParameter::new(0, universe)
            );
            channels.insert(
                PropertyType::Color(ColorPropertyType::Green), ChannelParameter::new(1, universe)
            );
            channels.insert(
                PropertyType::Color(ColorPropertyType::Blue), ChannelParameter::new(2, universe)
            );

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

            let value = match crate::cli::command_parsing::parse_cli_value(&*value) {
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

/// Executes an implicit CLI action sent from a client, returning a structured [`CliActionResponse`].
///
/// # Arguments
///
/// * `cli_action` - Reference to the [`CliAction`] to process implicitly
pub(crate) fn execute_implicit_cli_action(cli_action: &CliAction) -> CliActionResponse {
    match cli_action {
        CliAction::Help => {
            r_debug_log!(Warning, "A Client has sent an implicit Help-Command. Please dont let him do that.");
            UnsupportedCommand
        }

        CliAction::FixtureNew { name, channels } => {
            if let Err(e) = FixtureType::new(name.clone(), channels.clone()) {
                r_debug_log!(Warning, "Implicit FixtureNew threw an (User-)Error: {:?}", e);
                FixtureError(e)
            } else {
                Ack
            }
        }

        CliAction::FixtureAdd { name, fixture_type_name, universe, channel } => {
            if let Err(e) = fixture::new_fixture(name.clone(), fixture_type_name.clone(), *channel, *universe) {
                r_debug_log!(Warning, "Implicit FixtureAdd threw an (User-)Error: {:?}", e);
                FixtureError(e)
            } else {
                Ack
            }
        }

        CliAction::FixtureMove { fixture_name, new_universe, new_channel } => {
            if let Err(e) = fixture::move_fixture(fixture_name.clone(), *new_channel, *new_universe) {
                r_debug_log!(Warning, "Implicit FixtureMove threw an (User-)Error: {:?}", e);
                FixtureError(e)
            } else {
                Ack
            }
        }

        CliAction::FixtureRemove { fixture_name } => {
            if let Err(e) = fixture::remove_fixture(fixture_name.clone()) {
                r_debug_log!(Warning, "Implicit FixtureRemove threw an (User-)Error: {:?}", e);
                FixtureError(e)
            } else {
                Ack
            }
        }

        CliAction::FixtureSet {name, property_type, value} => {
            if let Err(e) = fixture::set_property(name.clone(), property_type.clone(), *value) {
                r_debug_log!(Warning, "Implicit FixtureSet threw an (User-)Error: {:?}", e);
                FixtureError(e)
            } else {
                Ack
            }
        }

        CliAction::FixtureGetType { fixture_name } => {
            match fixture::get_fixture_type(fixture_name.clone()) {
                Ok(fixture_type) => FixtureTypeInfo(fixture_type),
                Err(e) => {
                    r_debug_log!(Warning, "Implicit FixtureGetType threw an (User-)Error: {:?}", e);
                    FixtureError(e)
                },
            }
        }

        CliAction::Exit {..} => {
            r_debug_log!(Warning, "{}", "A client has sent an implicit Exit-Command. Only kernel can exit a session");
            UnsupportedCommand
        }

        CliAction::OtherCommands { command } => {
            r_debug_log!(Warning, "A client has sent an implicit Debug-Command : {}. Please dont him do that.", command);
            UnsupportedCommand
        }
    }
}

/// Creates a new fixture type definition with the given name and channel mappings.
///
/// # Arguments
///
/// * `name`     - The name of the new fixture type
/// * `channels` - A map linking property types to their channel parameters
fn new_fixture_type(name: String, channels: HashMap<PropertyType, ChannelParameter>) -> (LogLevel, String) {
    match FixtureType::new(name.clone(), channels) {

        Err(ChannelError(ChannelAlreadyInUse(channel_type))) => {
            (UserError,format!("Error: The channel {channel_type} overlaps with another channel."))
        }

        Err(ChannelError(ChannelOutOfRange)) => {
            (UserError,"Error: A Channel is higher than the size of the Universe. This is not yet supported".into())
        }

        Err(FixtureTypeNameAlreadyInUse(name)) => {
            (UserError, format!("Error: The Fixture type name {name} is already used."))
        }

        Err(InvalidPropertyType(property_type)) => {
            (UserError,format!("Error: \"{property_type}\" is not a valid PropertyType"))
        }

        Err(MultipleColorOutputTypes(error_message)) => {
            (UserError,error_message)
        }

        Err(_) => {
            debug_panic_or_return_log!("new_fixture_type() threw an Error it shouldn't")
        }

        Ok(()) => {
            (UserSuccess,format!("{} created successfully", name))
        }
    }
}

/// Spawns a new fixture instance based on an existing fixture type at the specified universe and channel.
///
/// # Arguments
///
/// * `name`              - The name/identifier for the new fixture instance
/// * `fixture_type_name` - The name of the fixture type to instantiate
/// * `universe`          - The target universe index
/// * `channel`           - The starting channel index
fn new_fixture(name: String, fixture_type_name: String, universe: usize, channel: ChannelIndex,)
    -> (LogLevel, String) {
    match fixture::new_fixture(name.clone(), fixture_type_name, channel, universe) {

        Err(FixtureNameAlreadyInUse(name)) => {
            (UserError,format!("Error: The Fixture name {name} is already used."))
        }

        Err(InvalidFixtureType(fixture_type_name)) => {
            (UserError,format!("Error: There is no fixture-type named \"{fixture_type_name}\"."))
        }

        Err(ChannelError(ChannelAlreadyInUse(overlapping_fixture))) => {
            (UserError,format!(
                "Error: At least one Channel of this fixture is overlapping with {}. Fixture has not been created.",
                overlapping_fixture
            ))
        }

        Err(ChannelError(ChannelOutOfRange)) => {
            (UserError,"Error: fixture overflows out of this remaining universe".to_string())
        }

        Err(ChannelError(UniverseOutOfRange)) => {
            debug_panic_or_return_log!(
                "Fatal Error: Fixture created in Universe that does not exist. Normally, the programm should \
                automatically create an universe, but somehow, this hasn't happened"
            )
        }

        Err(DmxStateDesync) => {
            debug_panic_or_return_log!(
                "Fatal Error: Dmx-State has desynced from real Fixture-Positions"
            )
        }

        Err(_) => {
            debug_panic_or_return_log!("new_fixture_type() threw an Error it shouldn't")
        }

        Ok(_) => {
            (UserSuccess,format!("{} created successfully", name))
        }
    }
}

/// Relocates an existing fixture instance to a new universe and channel position.
///
/// # Arguments
///
/// * `fixture_name`  - The name of the fixture to move
/// * `new_universe`  - The destination universe index
/// * `new_channel`   - The destination starting channel index
fn move_fixture(fixture_name: String, new_universe: usize, new_channel: ChannelIndex) -> (LogLevel, String) {
    match fixture::move_fixture(fixture_name.clone(), new_channel, new_universe) {

        Err(InvalidFixture(fixture_name)) => {
            (UserError,format!("Error: There is no fixture named \"{fixture_name}\"."))
        }

        Err(ChannelError(ChannelAlreadyInUse(overlapping_fixture))) => {
            (UserError,format!(
                "Error: At least one Channel of this fixture is overlapping with {}. Fixture has not been created.",
                overlapping_fixture
            ))
        }

        Err(ChannelError(ChannelOutOfRange)) => {
            (UserError,"Error: fixture overflows out of this remaining universe".to_string())
        }

        Err(ChannelError(UniverseOutOfRange)) => {
            debug_panic_or_return_log!(
                "Fatal Error: Fixture created in Universe that does not exist. Normally, the programm should \
                automatically create an universe, but somehow, this hasn't happened"
            )
        }

        Err(DmxStateDesync) => {
            debug_panic_or_return_log!(
                "Fatal Error: Dmx-State has desynced from real Fixture-Positions"
            )
        }

        Err(_) => {
            debug_panic_or_return_log!("new_fixture_type() threw an Error it shouldn't")
        }

        Ok(_) => {
            (UserSuccess,format!("{} moved successfully", fixture_name))
        }
    }
}

/// Removes an existing fixture instance.
///
/// # Arguments
///
/// * `fixture_name` - The name of the fixture to remove
fn remove_fixture(fixture_name: String) -> (LogLevel, String) {
    match fixture::remove_fixture(fixture_name.clone()) {

        Err(InvalidFixture(fixture_name)) => {
            (UserError,format!("Error: There is no fixture named \"{fixture_name}\""))
        }

        Err(ChannelError(UniverseOutOfRange)) => {
            debug_panic_or_return_log!(
                "Fatal Error: Fixture {} is in a non-existent Universe", fixture_name
            )
        }

        Err(DmxStateDesync) => {
            debug_panic_or_return_log!(
                "Fatal Error: Dmx-State has desynced from real Fixture-Positions"
            )
        }

        Err(_) => {
            debug_panic_or_return_log!("new_fixture_type() threw an Error it shouldn't")
        }

        Ok(_) => {
            (UserSuccess,format!("{} removed successfully", fixture_name))
        }
    }
}

/// Updates a specific property value on a target fixture instance.
///
/// # Arguments
///
/// * `fixture_name`  - The name of the fixture to update
/// * `property_type` - The property type to modify
/// * `value`         - The new channel value to assign
fn set_property_value(fixture_name: String, property_type: PropertyType, value: ChannelValue) -> (LogLevel, String) {
    match fixture::set_property(fixture_name.clone(), property_type.clone(), value) {
        Err(InvalidFixture(name)) => {
            (UserError,format!("Error: \"{name}\" is not a valid Fixture"))
        }

        Err(MissingProperty(_)) => {
            (UserError,format!("Error: \"{fixture_name}\" has no property \"{property_type}\""))
        }

        Err(_) => {
            debug_panic_or_return_log!("new_fixture_type() threw an Error it shouldn't")
        }

        Ok(_) => {
            (UserSuccess,format!("Value {property_type} of {fixture_name} changed successfully to {value}"))
        }
    }
}

/// Queries and retrieves the type name associated with a specific fixture instance.
///
/// # Arguments
///
/// * `fixture_name` - The name of the fixture to query
fn get_fixture_type(fixture_name: String) -> (LogLevel, String) {
    match fixture::get_fixture_type(fixture_name.clone()) {

        Err(InvalidFixture(fixture)) => {
            (UserError,format!("Error: \"{fixture}\" is not a valid Fixture"))
        }

        Err(_) => {
            debug_panic_or_return_log!("get_fixture_type_from_string() threw an Error it shouldn't")
        }

        Ok(fixture_type) => {
            (Info,format!("\"{fixture_name}\" is a fixture of the type \"{fixture_type}\""))
        }
    }
}


/// Handles kernel session shutdown procedures, optionally prompting the user to save changes if unspecified.
///
/// # Arguments
///
/// * `save_changes` - Optional boolean flag indicating whether to save changes (`Some(true)` / `Some(false)`) or prompt the user (`None`)
fn shutdown_kernel(save_changes: &Option<bool>) {
    r_log!(Info,"Shutting down Kernel");

    let (should_exit, save_changes) = match save_changes {
        Some(save_changes) => (true, *save_changes),
        None => {
            let stdout = io::stdout();
            let mut handle = stdout.lock();

            write!(handle, "\r\x1B[2K").unwrap();
            write!(handle, "\x1b[33m[System] Do you want to save before exiting?\n\
            (1) Save and exit (Warning: Not yet implemented. Its impossible to save right now)\n\
            (2) Discard and exit\n\
            (0) Cancel\n>\
            \x1b[0m").unwrap();

            handle.flush().unwrap();

            let mut exit_choice = String::new();
            io::stdin().read_line(&mut exit_choice).unwrap();
            match exit_choice.trim() {
                "1" => (true, true),
                "2" => (true, false),
                _ => {
                    writeln!(handle, "\x1b[32m[System] Exit canceled. Resuming kernel...\x1b[0m").unwrap();
                    (false, false)
                }
            }
        }
    };

    if should_exit {
        if save_changes {
            r_log!(Warning, "Couldnt save changes ... not yet implemented");
        } else {
            r_log!(Warning, "Exiting without saving changes");
        }

        announce_shutdown();

        // Let in "lock-que" waiting logging-messages get to their turn, and the TCP have his last Fun.
        std::thread::sleep(std::time::Duration::from_millis(500));

        std::process::exit(0);
    }
}

/// Macro that either triggers a panic in debug mode (outside tests) or returns an error log tuple.
#[macro_export]
macro_rules! debug_panic_or_return_log {
    ($($arg:tt)*) => {{
        #[cfg(all(debug_assertions, not(test)))]
        panic!("{}", format!($($arg)*));

        #[cfg(not(all(debug_assertions, not(test))))]
        (LogLevel::Error, format!($($arg)*))
    }};
}
