use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use serde::{Deserialize, Serialize};
use crate::fixture::{ChannelError, ChannelIndex, ChannelParameter, ChannelValue, FixtureError, PropertyType};

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

    FixtureMove {
        fixture_name: String,
        new_universe: usize,
        new_channel: ChannelIndex,
    },

    FixtureRemove {
        fixture_name: String,
    },

    FixtureSet {
        name: String,
        property_type: PropertyType,
        value: ChannelValue,
    },

    FixtureGetType {
        fixture_name: String,
    },

    Exit {
        save_changes: Option<bool>,
    },

    OtherCommands {
        command: String,
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum CliActionResponse {
    Ack,

    FixtureTypeInfo(String),

    FixtureError(FixtureError),

    ChannelError(ChannelError),

    UnsupportedCommand
}

impl Display for CliAction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CliAction::Help =>
                write!(f, "help"),
            CliAction::FixtureNew { name, channels } =>
                write!(f, "new {} {:?}", name, channels),
            CliAction::FixtureAdd { name, fixture_type_name, universe, channel } =>
                write!(f, "add {} {} {} {}", name, fixture_type_name, universe, channel),
            CliAction::FixtureMove { fixture_name, new_universe, new_channel } =>
                write!(f, "move {} {} {}", fixture_name, new_universe, new_channel),
            CliAction::FixtureRemove { fixture_name } =>
                write!(f, "remove {}", fixture_name),
            CliAction::FixtureSet { name, property_type, value } =>
                write!(f, "set {} {} {}", name, property_type, value),
            CliAction::FixtureGetType {fixture_name} =>
                write!(f, "type {}", fixture_name),
            CliAction::Exit {save_changes} =>
                write!(f, "exit {:?}", save_changes),
            CliAction::OtherCommands {command} =>
                write!(f, "{}", command),
        }
    }
}

impl Display for CliActionResponse {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CliActionResponse::Ack => write!(f, "ack"),
            CliActionResponse::FixtureTypeInfo(type_info) => write!(f, "FixtureTypeInfo {}", type_info),
            CliActionResponse::FixtureError(e) => write!(f, "FixtureError {:?}", e),
            CliActionResponse::ChannelError(e) => write!(f, "ChannelError {:?}", e),
            CliActionResponse::UnsupportedCommand => write!(f, "Unsupported Command"),
        }
    }
}

