use std::collections::HashMap;
use std::io;
use std::io::Write;
use crate::r_log;
use crate::networking::{announce_shutdown, on_dmx_config_update};
use crate::cli::command_parsing::parse_debug_command;
use common::cli_actions::{CliAction, CliActionResponse};
use common::cli_actions::CliActionResponse::{Ack, FixtureError, FixtureTypeInfo, UnsupportedCommand};
use common::logging::LogLevel;
use common::logging::LogLevel::{Error, Info, UserError, UserSuccess, Warning};
use common::fixture::{ChannelIndex, ChannelValue, ChannelParameter, PropertyType, FixtureType, Fixture, get_dmx_config_for_client};
use common::fixture::ChannelError::{ChannelAlreadyInUse, ChannelOutOfRange, UniverseOutOfRange};
use common::fixture::FixtureError::{ChannelError, FixtureNameAlreadyInUse, FixtureTypeNameAlreadyInUse, InvalidFixture, InvalidFixtureType, InvalidPropertyType, MissingProperty, MultipleColorOutputTypes};

pub fn execute_cli_action(is_kernel: bool, cli_action: &CliAction) -> (LogLevel, String) {
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
            parse_debug_command(command.clone())
        },

        //_ => (Error, "Not yet implemented".to_string())
    }
}

pub fn execute_implicit_cli_action(cli_action: &CliAction) -> CliActionResponse {
    match cli_action {
        CliAction::Help => {
            r_log!(Warning, "A Client has sent an implicit Help-Command. Please dont let him do that.");
            UnsupportedCommand
        }

        CliAction::FixtureNew { name, channels } => {
            if let Err(e) = FixtureType::new(name.clone(), channels.clone()) {
                FixtureError(e)
            } else {
                Ack
            }
        }

        CliAction::FixtureAdd { name, fixture_type_name, universe, channel } => {
            if let Err(e) = Fixture::new(fixture_type_name.clone(), *channel, *universe, name.clone()) {
                FixtureError(e)
            } else {
                Ack
            }
        }

        CliAction::FixtureSet {name, property_type, value} => {
            if let Err(e) = Fixture::set(name.clone(), property_type.clone(), *value) {
                FixtureError(e)
            } else {
                Ack
            }
        }

        CliAction::FixtureGetType { fixture_name } => {
            match Fixture::get_fixture_type_from_string(fixture_name.clone()) {
                Ok(fixture_type) => FixtureTypeInfo(fixture_type),
                Err(e) => FixtureError(e),
            }
        }

        CliAction::Exit {..} => {
            r_log!(Warning, "{}", "A client has sent an implicit Exit-Command. Only kernel can exit a session");
            UnsupportedCommand
        }

        CliAction::OtherCommands { command } => {
            r_log!(Warning, "A client has sent an implicit Debug-Command : {}. Please dont him do that.", command);
            UnsupportedCommand
        }
    }
}


pub(crate) fn new_fixture_type(name: String, channels: HashMap<PropertyType, ChannelParameter>) -> (LogLevel, String) {


    let fixture_type = FixtureType::new(name.clone(), channels);
    match fixture_type {
        Err(ChannelError(ChannelAlreadyInUse(channel_type))) => {
            (UserError,format!("Error: The channel {channel_type} overlaps with another channel."))
        }

        Err(ChannelError(ChannelOutOfRange)) => {
            (UserError,"Error: A Channel is higher than the size of the Universe. This is not yet supported".to_string())
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
            r_log!(Error,"new_fixture_type() threw an Error it shouldn't");
            None::<Fixture>.unwrap();
            unreachable!();
            // Mir ist langweilig, deswegen crashe ich hier, auf die lustigste und verwirrendste Art. Hier muss auch
            // gecrashed werden, weil das nie passieren sollte
        }

        Ok(()) => {
            (UserSuccess,format!("{} created successfully", name))
        }
    }
}

fn new_fixture(name: String, fixture_type_name: String, universe: usize, channel: ChannelIndex,)
    -> (LogLevel, String) {

    let fixture = Fixture::new(fixture_type_name, channel, universe, name.clone());
    match fixture {
        Err(ChannelError(ChannelOutOfRange)) => {
            (UserError,"Error: fixture overflows out of this remaining universe".to_string())
        }

        Err(ChannelError(UniverseOutOfRange)) => {
            panic!(
                "Fatal Error: Fixture created in Universe that does not exist. Normally, the programm should \
        automatically create an universe, but somehow, this hasn't happened"
            );
        }

        Err(ChannelError(ChannelAlreadyInUse(overlapping_fixture))) => {
            (UserError,format!(
                "Error: At least one Channel of this fixture is overlapping with {}. Fixture has not been created.",
                overlapping_fixture
            ))
        }

        Err(InvalidFixtureType(fixture_type_name)) => {
            (UserError,format!("Error: There is no fixture-type named \"{fixture_type_name}\"."))
        }

        Err(FixtureNameAlreadyInUse(name)) => {
            (UserError,format!("Error: The Fixture name {name} is already used."))
        }

        Err(_) => {
            r_log!(Error, "new_fixture_type() threw an Error it shouldn't");
            None::<Fixture>.unwrap();
            unreachable!()
            // Mir ist langweilig, deswegen crashe ich hier, auf die lustigste und verwirrendste Art. Hier muss auch
            // gecrashed werden, weil das nie passieren sollte, und ich hab all das einfach von new_fixture_type kopiert
        }

        Ok(_) => {
            let client_state = get_dmx_config_for_client();
            on_dmx_config_update(client_state);
            (UserSuccess,format!("{} created successfully", name))
        }
    }
}

fn set_property_value(fixture_name: String, property_type: PropertyType, value: ChannelValue) -> (LogLevel, String) {
    let result = Fixture::set(fixture_name.clone(), property_type.clone(), value);
    match result {
        Err(InvalidPropertyType(property_type)) => {
            (UserError,format!("Error: \"{property_type}\" is not a valid PropertyType"))
        }

        Err(MissingProperty(_)) => {
            (UserError,format!("Error: \"{fixture_name}\" has no property \"{property_type}\""))
        }

        Err(InvalidFixture(name)) => {
            (UserError,format!("Error: \"{name}\" is not a valid Fixture"))
        }

        Err(_) => {
            r_log!(Error, "new_fixture_type() threw an Error it shouldn't");
            None::<Fixture>.unwrap();
            unreachable!()
            // Mir ist langweilig, deswegen crashe ich hier, auf die lustigste und verwirrendste Art. Hier muss auch
            // gecrashed werden, weil das nie passieren sollte, und ich hab all das einfach schon wieder von
            // new_fixture_type kopiert
        }

        Ok(_) => {
            (UserSuccess,format!("Value {property_type} of {fixture_name} changed successfully to {value}"))
        }
    }
}

fn get_fixture_type(fixture_name: String) -> (LogLevel, String) {
    match Fixture::get_fixture_type_from_string(fixture_name.clone()) {
        Ok(fixture_type) =>
            (Info,format!("\"{fixture_name}\" is a fixture of the type \"{fixture_type}\"")),
        Err(InvalidFixture(fixture)) =>
            (UserError,format!("Error: \"{fixture}\" is not a valid Fixture")),
        Err(_) =>
            panic!("Error: get_fixture_type_from_string() threw an Error it shouldn't"),
    }
}

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
