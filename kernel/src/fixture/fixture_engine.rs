use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::{mpsc, OnceLock};
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use common::fixture::{ChannelIndex, ChannelValue, Fixture, FixtureError, PropertyType, MAX_CHANNEL};
use common::fixture::ChannelError::{ChannelAlreadyInUse, UniverseOutOfRange};
use crate::fixture::fixture_command::FixtureCommand;
use common::fixture::FixtureError::{DmxStateDesync, InvalidFixture};
use common::logging::LogLevel::{Error, Info};
use common::networking::subscription_objects::{DMXConfigForClientState, DMXConfigurationForClient};
use common::{r_debug_log, r_log};
use crate::fixture::fixture_engine::ChannelReservation::{Empty, Pending, Reserved};
use crate::networking::on_dmx_config_update;

/// Represents the reservation state of a single DMX-Channel.
///
/// * **Empty** – Channel is not in use.
/// * **Pending(String)** – Channel has been claimed by a fixture but not yet finalized.
/// * **Reserved(String, PropertyType, usize)** – Channel is fully reserved by a fixture with an associated property.
#[derive(Clone, Debug)]
enum ChannelReservation {
    Empty,
    Pending(String),
    Reserved(String, PropertyType, usize),
}

/// The core engine managing fixture instances, thread synchronization, and DMX channel reservations.
pub struct FixtureEngine {
    fixtures: HashMap<String, Fixture>,
    dmx_config: Vec<[ChannelReservation; MAX_CHANNEL as usize]>,
    receiver: Receiver<FixtureCommand>,
}

/// Global static sender channel used to dispatch asynchronous commands to the running `FixtureEngine`.
static FIXTURE_ACTION_SENDER: OnceLock<Sender<FixtureCommand>> = OnceLock::new();

impl FixtureEngine {

    /// Spawns the fixture engine actor thread and returns an interface receiver channel for updates.
    ///
    /// # Errors
    ///
    /// Returns an error if the engine has already been started (preventing multiple instances).
    pub fn spawn() -> Result<Receiver<(usize,Vec<Fixture>)>, &'static str > {

        if FIXTURE_ACTION_SENDER.get().is_some() {
            return Err("Critical Error: The fixture engine has already been started!");
        }

        let (tx, rx) = mpsc::channel();

        let mut engine = Self {
            fixtures: HashMap::new(),
            dmx_config: Vec::new(),
            receiver: rx,
        };

        if FIXTURE_ACTION_SENDER.set(tx).is_err() {
            return Err("Race condition: Engine was started in parallel!");
        }

        let (interface_sender, interface_receiver) = mpsc::channel();

        thread::spawn(move || {
            engine.run(interface_sender)
        });

        r_debug_log!(Info, "Fixture Engine Actor thread started successfully.");

        Ok(interface_receiver)
    }

    /// Main event loop processing incoming commands and triggering state notifications.
    ///
    /// # Arguments
    ///
    /// * `interface_sender` - Channel used to broadcast updated fixture lists and universe counts to the runtime.
    fn run(&mut self, interface_sender: Sender<(usize,Vec<Fixture>)>) {
        while let Ok(command) = self.receiver.recv() {
            let mut dmx_config_changed = false;
            let mut dmx_values_changed = false;
            match command {
                FixtureCommand::SpawnFixture {name, fixture_type_name, start_channel, start_universe, reply_to } => {
                    let result = self.new_fixture(name, fixture_type_name, start_channel, start_universe);
                    reply_to.send(result.clone()).unwrap();
                    if result.is_ok() {
                        dmx_config_changed = true;
                    }
                }

                FixtureCommand::MoveFixture {name, new_channel, new_universe, reply_to} => {
                    let result = self.move_fixture(name, new_channel, new_universe);
                    reply_to.send(result.clone()).unwrap();
                    if result.is_ok() {
                        dmx_config_changed = true;
                    }
                }

                FixtureCommand::RemoveFixture {name, reply_to} => {
                    let result = self.remove_fixture(name);
                    reply_to.send(result.clone()).unwrap();
                    if result.is_ok() {
                        dmx_config_changed = true;
                    }
                }

                FixtureCommand::SetProperty {fixture_name, property, value, reply_to} => {
                    let result = self.set_property(fixture_name, property, value);
                    reply_to.send(result.clone()).unwrap();
                    if result.is_ok() {
                        dmx_values_changed = true;
                    }
                }

                FixtureCommand::GetType {fixture_name, reply_to} => {
                    reply_to.send(self.get_fixture_type_from_string(fixture_name)).unwrap();
                }
            }

            if dmx_config_changed {
                dmx_values_changed = true;
                on_dmx_config_update(self.get_dmx_config_for_client());
            }

            if dmx_values_changed {
                let universe_count = self.dmx_config.len();
                let fixtures: Vec<Fixture> = self.fixtures.values().cloned().collect();
                interface_sender.send((universe_count, fixtures)).unwrap();
            }

        }
    }

    /// Creates and registers a new fixture instance, verifying and reserving its DMX channels.
    ///
    /// # Arguments
    ///
    /// * `name`              - Unique name for the fixture instance
    /// * `fixture_type_name` - Template name of the registered fixture type
    /// * `start_channel`     - DMX channel offset within the universe
    /// * `start_universe`    - Target DMX universe index (0-based)
    ///
    /// # Errors
    ///
    /// * [`FixtureNameAlreadyInUse`](FixtureError::FixtureNameAlreadyInUse) – if the name is already taken
    /// * [`InvalidFixtureType`](FixtureError::InvalidFixtureType) - if `fixture_type_name` is not registered
    /// * [`ChannelAlreadyInUse`] – if required channels overlap with existing
    /// * [`ChannelOutOfRange`](ChannelError::ChannelOutOfRange) - if any required channel is out of bounds
    /// ([MAX_CHANNEL]).
    /// * [`UniverseOutOfRange`] - if the fixture is in a non-existent universe
    /// * [`DmxStateDesync`] - if the DMX-State and the fixture registry are out of sync
    fn new_fixture(
        &mut self, name: String, fixture_type_name: String, start_channel: ChannelIndex, start_universe: usize
    ) -> Result<(), FixtureError> {
        let fixture = Fixture::new(
            fixture_type_name, start_channel,start_universe, name.clone()
        )?;

        let max_universe = fixture
            .iter_over_properties()
            .flat_map(|(_, channel)| channel.get_channel_indices())
            .map(|(_, universe_index)| universe_index)
            .max()
            .unwrap_or(start_universe);

        self.ensure_universe_size(max_universe + 1);

        //Pending Reservation
        fixture.iter_over_properties().try_for_each(|(_, channel)| {
            for (channel_index, universe_index) in channel.get_channel_indices() {
                let universe = self.dmx_config.get_mut(universe_index)
                    .ok_or(UniverseOutOfRange)?;
                if let Reserved(existing, property, _) = &universe[channel_index as usize] {
                    return Err(ChannelAlreadyInUse(format!("{}, {}", existing, property)));
                }

                universe[channel_index as usize] = Pending(name.clone())
            }

            Ok(())
        }).map_err(FixtureError::from)?;

        // Insert into fixture list
        if let Entry::Vacant(entry) = self.fixtures.entry(name.clone()) {
            entry.insert(fixture.clone());
        } else {
            return Err(FixtureError::FixtureNameAlreadyInUse(name.clone()))
        }

        //Finalize Reservation
        fixture.iter_over_properties().try_for_each(|(property, channel)| {
            let mut fine_degree = 0;

            for (channel_index, universe_index) in channel.get_channel_indices() {
                let universe = self.dmx_config.get_mut(universe_index)
                    .ok_or(UniverseOutOfRange)?;

                let ch_index = channel_index as usize;

                match &universe[ch_index] {
                    Pending(existing) if *existing == name => {
                        universe[ch_index] = Reserved(existing.clone(), property.clone(), fine_degree);
                        fine_degree += 1;
                    }

                    _ => {
                        return Err(DmxStateDesync);
                    }
                }
            }

            Ok(())
        }).map_err(FixtureError::from)?;

        Ok(())
    }

    /// Relocates an existing fixture to a new channel and/or universe.
    ///
    /// # Arguments
    ///
    /// * `name`          - Name of the fixture to move
    /// * `new_channel`   - New starting DMX channel index
    /// * `new_universe`  - New target DMX universe index (0-based)
    ///
    /// # Errors
    ///
    /// * [`InvalidFixture`] – if the fixture name is not found
    /// * [`ChannelAlreadyInUse`] – if the new target channels are blocked
    /// * [`ChannelOutOfRange`](ChannelError::ChannelOutOfRange) - if the new channel range is out of bounds
    /// * [`UniverseOutOfRange`] - if the fixture is in a non-existent universe
    /// * [`DmxStateDesync`] - if the DMX-State and the fixture registry are out of sync
    fn move_fixture(
        &mut self, name: String, new_channel: ChannelIndex, new_universe: usize
    ) -> Result<(), FixtureError> {
        let mut fixture_original = self.fixtures.get_mut(&name).ok_or(InvalidFixture(name.clone()))?
            .clone();


        let mut fixture_clone = fixture_original.clone();
        fixture_clone.move_to_channel(new_channel, new_universe)?;

        let max_universe = fixture_clone
            .iter_over_properties()
            .flat_map(|(_, channel)| channel.get_channel_indices())
            .map(|(_, uni_idx)| uni_idx)
            .max()
            .unwrap_or(new_universe);

        self.ensure_universe_size(max_universe + 1);

        // Reserve Pending
        fixture_clone.iter_over_properties().try_for_each(|(_, channel)| {

            for (channel_index, universe_index) in channel.get_channel_indices() {
                let universe = self.dmx_config.get_mut(universe_index)
                    .ok_or(UniverseOutOfRange)?;

                match &universe[channel_index as usize] {
                    Reserved(existing, _, _) if existing == &name => {}
                    Reserved(existing, property, _) => {
                        return Err(ChannelAlreadyInUse(format!("{}, {}", existing, property)));
                    }
                    _ => {
                        universe[channel_index as usize] = Pending(name.clone())
                    }
                }
            }

            Ok(())
        }).map_err(FixtureError::from)?;

        // Remove old Reservations
        self.remove_reservations(&mut fixture_original)?;

        // Reserve final
        fixture_clone.iter_over_properties().try_for_each(|(property, channel)| {
            let mut fine_degree = 0;

            for (channel_index, universe_index) in channel.get_channel_indices() {
                let universe = self.dmx_config.get_mut(universe_index)
                    .ok_or(UniverseOutOfRange)?;

                let ch_index = channel_index as usize;

                match &universe[ch_index] {
                    Pending(existing) if *existing == name => {
                        universe[ch_index] = Reserved(name.clone(), property.clone(), fine_degree);
                        fine_degree += 1;
                    }

                    Empty => {
                        universe[ch_index] = Reserved(name.clone(), property.clone(), fine_degree);
                        fine_degree += 1;
                    }

                    _ => {
                        return Err(DmxStateDesync)
                    }
                }
            }

            Ok(())
        }).map_err(FixtureError::from)?;

        self.fixtures.insert(name, fixture_clone);

        Ok(())
    }

    /// Removes a fixture instance and frees its associated DMX channel reservations.
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the fixture to remove
    ///
    /// # Errors
    ///
    /// * [`InvalidFixture`] – if the fixture does not exist
    /// * [`UniverseOutOfRange`] - if the fixture is in a non-existent universe
    /// * [`DmxStateDesync`] - if the DMX-State and the fixture registry are out of sync
    fn remove_fixture(&mut self, name: String) -> Result<(), FixtureError> {
        let mut fixture = self.fixtures.remove(&name).ok_or(InvalidFixture(name.clone()))?;

        if let Err(e) = self.remove_reservations(&mut fixture) {

            //Rollback
            self.fixtures.insert(name, fixture);
            return Err(e);
        }

        Ok(())
    }

    /// Updates a specific property value on an active fixture.
    ///
    /// # Arguments
    ///
    /// * `name`     - Name of the target fixture
    /// * `property` - Property type to update
    /// * `value`    - New raw channel value
    ///
    /// # Errors
    ///
    /// * [`InvalidFixture`] – if the fixture is not found
    /// * [`MissingProperty`](FixtureError::MissingProperty) – if the fixture lacks the specified property
    fn set_property(&mut self, name: String, property: PropertyType, value: ChannelValue) -> Result<(), FixtureError> {

        let fixture = self.fixtures.get_mut(&name).ok_or(InvalidFixture(name.clone()))?;

        fixture.set(property, value)?;

        Ok(())
    }

    /// Retrieves the fixture type name for a given fixture instance string.
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the fixture instance
    ///
    /// # Errors
    ///
    /// * [`InvalidFixture`] – if no fixture with this name exists
    fn get_fixture_type_from_string(&self, name: String) -> Result<String, FixtureError> {
        match self.fixtures.get(&name) {
            None => Err(InvalidFixture(name)),
            Some(fixture) => Ok(fixture.get_fixture_type()),
        }
    }

    /// Ensures that the internal DMX configuration vector has at least the given size (universes).
    ///
    /// # Arguments
    ///
    /// * `size` - Required minimum number of universes
    fn ensure_universe_size(&mut self, size: usize) {
        if size > self.dmx_config.len() {
            self.dmx_config.resize_with(size, || std::array::from_fn(|_| Empty));
        }
    }

    /// Clears all DMX channel reservations for a specific fixture.
    ///
    /// # Arguments
    ///
    /// * `fixture` - Reference to the fixture instance
    ///
    /// # Errors
    ///
    /// * [`UniverseOutOfRange`] - if the fixture is in a non-existent universe
    /// * [`DmxStateDesync`] - if the DMX-State and the fixture registry are out of sync
    fn remove_reservations(&mut self, fixture: &mut Fixture) -> Result<(), FixtureError> {
        let name = fixture.get_name();

        fixture.iter_over_properties().try_for_each(|(_, channel)| {
            for (channel_index, universe_index) in channel.get_channel_indices() {
                let universe = self.dmx_config.get_mut(universe_index)
                    .ok_or(UniverseOutOfRange)?;

                match &universe[channel_index as usize] {
                    Reserved(existing, _, _) if *existing == *name => {}
                    _ => {
                        return Err(DmxStateDesync);
                    }
                }
                universe[channel_index as usize] = Empty;
            }

            Ok(())
        }).map_err(FixtureError::from)?;
        Ok(())
    }

    /// Generates a snapshot of the current DMX configuration mapped for client subscription updates.
    fn get_dmx_config_for_client(&self) -> DMXConfigForClientState{
        self.dmx_config.iter().map(|universe| {
            universe.iter().map(|channel| {
                match channel {
                    Reserved(fixture, property, fine_degree) => {
                        let fixture_type = match self.fixtures.get(&fixture.clone()) {
                            Some(fixture_object) => fixture_object.get_fixture_type(),
                            None => {
                                r_log!(Error,"Fixture {} is saved in DMXConfiguration, but not in  FixtureList.",
                                        fixture
                                    );
                                return DMXConfigurationForClient::Empty;
                            }
                        };

                        let mut hasher = DefaultHasher::new();
                        fixture_type.hash(&mut hasher);
                        let full_hash: u64 = hasher.finish();
                        let fixture_type_hash = (full_hash % 256) as u8;

                        DMXConfigurationForClient::Reserved {
                            fixture_name: fixture.clone(),
                            property_type: property.clone(),
                            fine_degree: *fine_degree,
                            fixture_type_hash
                        }
                    }
                    _ => DMXConfigurationForClient::Empty,
                }
            }).collect()
        }).collect()
    }
}

/// Creates and registers a new fixture instance, verifying and reserving its DMX channels.
///
/// # Arguments
///
/// * `name`              - Unique name for the fixture instance
/// * `fixture_type_name` - Template name of the registered fixture type
/// * `start_channel`     - DMX channel offset within the universe
/// * `start_universe`    - Target DMX universe index (0-based)
///
/// # Errors
///
/// * [`FixtureNameAlreadyInUse`](FixtureError::FixtureNameAlreadyInUse) – if the name is already taken
/// * [`InvalidFixtureType`](FixtureError::InvalidFixtureType) - if `fixture_type_name` is not registered
/// * [`ChannelAlreadyInUse`] – if required channels overlap with existing
/// * [`ChannelOutOfRange`](ChannelError::ChannelOutOfRange) - if any required channel is out of bounds
/// ([MAX_CHANNEL]).
/// * [`UniverseOutOfRange`] - if the fixture is in a non-existent universe
/// * [`DmxStateDesync`] - if the DMX-State and the fixture registry are out of sync

pub fn new_fixture(
    name: String, fixture_type_name: String, start_channel: ChannelIndex, start_universe: usize
) -> Result<(), FixtureError> {
    let (reply_tx, reply_rx) = mpsc::channel();

    let cmd = FixtureCommand::SpawnFixture {
        name,
        fixture_type_name,
        start_channel,
        start_universe,
        reply_to: reply_tx,
    };

    let engine_tx = FIXTURE_ACTION_SENDER.get().expect("CRITICAL: Engine not running!");
    engine_tx.send(cmd).unwrap();

    reply_rx.recv().unwrap()
}

/// Relocates an existing fixture to a new channel and/or universe.
///
/// # Arguments
///
/// * `name`          - Name of the fixture to move
/// * `new_channel`   - New starting DMX channel index
/// * `new_universe`  - New target DMX universe index (0-based)
///
/// # Errors
///
/// * [`InvalidFixture`] – if the fixture name is not found
/// * [`ChannelAlreadyInUse`] – if the new target channels are blocked
/// * [`ChannelOutOfRange`](ChannelError::ChannelOutOfRange) - if the new channel range is out of bounds
/// * [`UniverseOutOfRange`] - if the fixture is in a non-existent universe
/// * [`DmxStateDesync`] - if the DMX-State and the fixture registry are out of sync
pub fn move_fixture(name: String, new_channel: ChannelIndex, new_universe: usize) -> Result<(), FixtureError> {
    let (reply_tx, reply_rx) = mpsc::channel();

    let cmd = FixtureCommand::MoveFixture {
        name,
        new_channel,
        new_universe,
        reply_to: reply_tx,
    };

    let engine_tx = FIXTURE_ACTION_SENDER.get().expect("CRITICAL: Engine not running!");
    engine_tx.send(cmd).unwrap();

    reply_rx.recv().unwrap()
}

/// Removes a fixture instance and frees its associated DMX channel reservations.
///
/// # Arguments
///
/// * `name` - Name of the fixture to remove
///
/// # Errors
///
/// * [`InvalidFixture`] – if the fixture does not exist
/// * [`UniverseOutOfRange`] - if the fixture is in a non-existent universe
/// * [`DmxStateDesync`] - if the DMX-State and the fixture registry are out of sync
pub fn remove_fixture(name: String) -> Result<(), FixtureError> {
    let (reply_tx, reply_rx) = mpsc::channel();

    let cmd = FixtureCommand::RemoveFixture {
        name,
        reply_to: reply_tx,
    };

    let engine_tx = FIXTURE_ACTION_SENDER.get().expect("CRITICAL: Engine not running!");
    engine_tx.send(cmd).unwrap();

    reply_rx.recv().unwrap()
}

/// Updates a specific property value on an active fixture.
///
/// # Arguments
///
/// * `fixture_name` - Name of the target fixture
/// * `property` - Property type to update
/// * `value` - New raw channel value
///
/// # Errors
///
/// * [`InvalidFixture`] – if the fixture is not found
/// * [`MissingProperty`](FixtureError::MissingProperty) – if the fixture lacks the specified property
pub fn set_property(fixture_name: String, property: PropertyType, value: ChannelValue) -> Result<(), FixtureError> {
    let (reply_tx, reply_rx) = mpsc::channel();

    let cmd = FixtureCommand::SetProperty {
        fixture_name,
        property,
        value,
        reply_to: reply_tx
    };

    let engine_tx = FIXTURE_ACTION_SENDER.get().expect("CRITICAL: Engine not running!");
    engine_tx.send(cmd).unwrap();

    reply_rx.recv().unwrap()
}

/// Retrieves the fixture type name for a given fixture instance string.
///
/// # Arguments
///
/// * `name` - Name of the fixture instance
///
/// # Errors
///
/// * [`InvalidFixture`] – if no fixture with this name exists
pub fn get_fixture_type(fixture_name: String) -> Result<String, FixtureError> {
    let (reply_tx, reply_rx) = mpsc::channel();

    let cmd = FixtureCommand::GetType {
        fixture_name,
        reply_to: reply_tx
    };

    let engine_tx = FIXTURE_ACTION_SENDER.get().expect("CRITICAL: Engine not running!");
    engine_tx.send(cmd).unwrap();

    reply_rx.recv().unwrap()
}