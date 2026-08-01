use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::cli::command_parsing::parse_debug_command;
use crate::fixture::{ChannelIndex, ChannelValue, ChannelParameter, PropertyType, FixtureType, Fixture};
use crate::fixture::ChannelError::{ChannelAlreadyInUse, ChannelOutOfRange, UniverseOutOfRange};
use crate::fixture::FixtureError::{ChannelError, FixtureNameAlreadyInUse, FixtureTypeNameAlreadyInUse, InvalidFixture, InvalidFixtureType, InvalidPropertyType, MissingProperty, MultipleColorOutputTypes};
use crate::logging::LogLevel;
use crate::logging::LogLevel::{Error, Info, UserError, UserSuccess};

#[derive(Serialize, Deserialize, Debug)]
pub enum CliAction {
    Help,

    FixtureNew {
        name: String,
        channels: HashMap<PropertyType, ChannelParameter>,
    },

    FixtureAdd {
        name: String,
        fixture_type_name: String,
        universe: usize,
        channel: ChannelIndex,
    },

    FixtureSet {
        name: String,
        property_type: PropertyType,
        value: ChannelValue,
    },

    FixtureGetType {
        fixture_name: String,
    },

    OtherCommands {
        command: String,
    }
}

impl CliAction {
    pub fn execute(&self) -> (LogLevel, String) {
        match self {
            CliAction::Help => {
                const HELP_TEXT: &str = include_str!("../../help.txt");
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

            CliAction::OtherCommands {command} => {
                parse_debug_command(command.clone())
            }

            //_ => (Error, "Not yet implemented".to_string())
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
