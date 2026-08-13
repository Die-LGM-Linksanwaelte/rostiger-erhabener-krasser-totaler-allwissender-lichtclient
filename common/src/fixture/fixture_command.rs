use std::sync::mpsc::Sender;
use crate::fixture::{ChannelIndex, ChannelValue, FixtureError, PropertyType};

pub enum FixtureCommand {
    SpawnFixture {
        name: String,
        fixture_type_name: String,
        start_channel: ChannelIndex,
        start_universe: usize,
        reply_to: Sender<Result<(), FixtureError>>,
    },

    MoveFixture {
        name: String,
        new_channel: ChannelIndex,
        new_universe: usize,
        reply_to: Sender<Result<(), FixtureError>>,
    },

    RemoveFixture {
        name: String,
        reply_to: Sender<Result<(), FixtureError>>,
    },

    SetProperty {
        fixture_name: String,
        property: PropertyType,
        value: ChannelValue,
        reply_to: Sender<Result<(), FixtureError>>,
    },

    GetType {
        fixture_name: String,
        reply_to: Sender<Result<String, FixtureError>>,
    }
}