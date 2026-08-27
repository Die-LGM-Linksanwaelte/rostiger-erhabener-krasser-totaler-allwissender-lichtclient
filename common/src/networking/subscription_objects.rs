use std::fmt;
use std::fmt::Formatter;
use serde::{Deserialize, Serialize};
use crate::fixture::PropertyType;

/// Represents the full DMX patch configuration mapped out for the client,
/// structured as a 2D array (universes containing individual channels).
pub type DMXConfigForClientState = Vec<Vec<DMXConfigurationForClient>>;

/// Defines the available data streams or engine states that a client can subscribe to.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum SubscribeTopic {
    /// The current DMX channel allocation and patch state.
    DMXConfiguration,
}

/// Container for the actual data payload dispatched during a topic update.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum TopicPayload {
    /// Payload carrying the synchronized DMX configuration state.
    DMXConfiguration(DMXConfigForClientState),
}

/// Specifies the frequency and condition under which the server transmits topic updates.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum UpdateMode {
    /// Updates are only transmitted when the underlying data actually changes.
    OnChange,
    /// Updates are transmitted continuously at a set interval, regardless of state changes.
    Continuous,
}

/// Represents the state of a single DMX channel tailored specifically for clear visual representation
/// in the client's GUI DMX overview.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum DMXConfigurationForClient {
    /// The DMX channel is currently unassigned.
    Empty,
    /// The DMX channel is allocated to a fixture property, providing necessary details for the UI.
    Reserved{
        /// The unique name of the fixture occupying this channel.
        fixture_name: String,
        /// The specific property type (e.g., Dimmer, Pan) mapped to this channel to display its function.
        property_type: PropertyType,
        /// The resolution layer of this channel (e.g., 0 for coarse, 1 for fine, 2 for ultra, ...).
        fine_degree: usize,
        /// A lightweight hash used by the GUI to assign consistent color coding to fixtures of the same type,
        /// keeping the DMX overview visually organized without cluttering it with full type names.
        fixture_type_hash: u8,
    },
}

impl TopicPayload {
    /// Retrieves the corresponding [`SubscribeTopic`] associated with this data payload.
    pub fn get_topic(&self) -> SubscribeTopic {
        match self {
            TopicPayload::DMXConfiguration(..) => SubscribeTopic::DMXConfiguration,
        }
    }
}


impl fmt::Display for SubscribeTopic {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            SubscribeTopic::DMXConfiguration => f.write_str("DMXConfiguration"),
        }
    }
}

