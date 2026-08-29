use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use serde::{Deserialize, Serialize};
use crate::fixture::{ChannelIndex, ChannelParameter, ChannelValue, FixtureError, PropertyType};

/// Represents the available command-line interface (CLI) actions that can be dispatched.
#[derive(Serialize, Deserialize, Debug)]
pub enum CliAction {
    /// Displays help information.
    Help,

    /// Creates a new custom fixture template type with specific properties.
    FixtureNew {
        /// Name of the new fixture type template.
        name: String,
        /// Mapping of property types to their respective channel parameters.
        channels: HashMap<PropertyType, ChannelParameter>,
    },

    /// Spawns an instance of a fixture into a specific universe and channel.
    FixtureAdd {
        /// Unique name for the fixture instance.
        name: String,
        /// Name of the registered fixture type template to use.
        fixture_type_name: String,
        /// Target DMX universe index.
        universe: usize,
        /// Starting DMX channel index within the universe.
        channel: ChannelIndex,
    },

    /// Relocates an existing fixture instance to a new universe and/or channel.
    FixtureMove {
        /// Name of the registered fixture instance to move.
        fixture_name: String,
        /// New target DMX universe index.
        new_universe: usize,
        /// New starting DMX channel index.
        new_channel: ChannelIndex,
    },

    /// Removes an existing fixture instance.
    FixtureRemove {
        /// Name of the fixture instance to remove.
        fixture_name: String,
    },

    /// Updates a specific property value of a fixture.
    FixtureSet {
        /// Name of the target fixture instance.
        name: String,
        /// The property type to update.
        property_type: PropertyType,
        /// New raw channel value to assign.
        value: ChannelValue,
    },

    /// Queries the fixture type name for a given fixture instance.
    FixtureGetType {
        /// Name of the fixture instance to query.
        fixture_name: String,
    },

    /// Shuts down the application, optionally saving configuration changes.
    ///
    /// **Behavior based on `save_changes`:**
    /// * `Some(false)` – Shuts down immediately without saving changes.
    /// * `Some(true)` – Saves changes and then shuts down.
    /// * `None` – Prompts the user interactively to confirm whether to save changes.
    ///
    /// **Note:** This command can only be executed within the kernel console.
    Exit {
        /// Optional flag indicating whether changes should be saved upon exit.
        save_changes: Option<bool>,
    },

    /// Fallback for custom or unparsed command strings, mainly used for debug-commands
    OtherCommands {
        /// The raw unparsed command string.
        command: String,
    }
}

/// Represents the response returned after the implicit executing of a [`CliAction`].
#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum CliActionResponse {
    /// Acknowledges successful execution without specific data.
    Ack,

    /// Returns metadata or type information about a fixture.
    FixtureTypeInfo(String),

    /// Returned when a fixture-related error occurs.
    FixtureError(FixtureError),

    /// Returned when the executed command is unrecognized or not supported via implicit command-executing.
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
            CliActionResponse::UnsupportedCommand => write!(f, "Unsupported Command"),
        }
    }
}

