use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::fixture::{ChannelIndex, ChannelParameter, ChannelValue, PropertyType};

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

