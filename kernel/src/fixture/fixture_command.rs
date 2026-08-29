use std::sync::mpsc::Sender;
use common::fixture::{ChannelIndex, ChannelValue, FixtureError, PropertyType};

/// Commands sent asynchronously to the fixture engine thread to manage fixture instances
/// and modify their properties or states.
pub(super) enum FixtureCommand {
    /// Spawns a new fixture instance with a given type, start address, and universe.
    SpawnFixture {
        /// Unique name for the new fixture instance.
        name: String,
        /// Name of the registered [`FixtureType`](common::fixture::FixtureType) template to use.
        fixture_type_name: String,
        /// Starting DMX-Channel index within the universe.
        start_channel: ChannelIndex,
        /// DMX universe index (0-based) where the fixture resides.
        start_universe: usize,
        /// Sender channel used to reply with the success or failure result.
        reply_to: Sender<Result<(), FixtureError>>,
    },

    /// Moves an existing fixture to a new DMX channel and/or universe and changes the reserved channels accordingly
    MoveFixture {
        /// Name of the fixture to move.
        name: String,
        /// New starting DMX channel index.
        new_channel: ChannelIndex,
        /// New target DMX universe index.
        new_universe: usize,
        /// Sender channel used to reply with the success or failure result.
        reply_to: Sender<Result<(), FixtureError>>,
    },

    /// Removes an existing fixture instance and frees its reserved channels.
    RemoveFixture {
        /// Name of the fixture to remove.
        name: String,
        /// Sender channel used to reply with the success or failure result.
        reply_to: Sender<Result<(), FixtureError>>,
    },

    /// Sets a specific property value on a target fixture.
    SetProperty {
        /// Name of the target fixture.
        fixture_name: String,
        /// The property type to modify (Simple or Color).
        property: PropertyType,
        /// The new channel value to assign.
        value: ChannelValue,
        /// Sender channel used to reply with the success or failure result.
        reply_to: Sender<Result<(), FixtureError>>,
    },

    /// Retrieves the fixture type name for a given fixture.
    GetType {
        /// Name of the target fixture.
        fixture_name: String,
        /// Sender channel used to reply with the fixture type name string or an error.
        reply_to: Sender<Result<String, FixtureError>>,
    }
}