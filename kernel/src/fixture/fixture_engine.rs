use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::{mpsc, OnceLock};
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use common::fixture::{ChannelError, ChannelIndex, ChannelValue, Fixture, FixtureError, PropertyType, MAX_CHANNEL};
use common::fixture::ChannelError::{ChannelAlreadyInUse, UniverseOutOfRange};
use common::fixture::fixture_command::FixtureCommand;
use common::fixture::FixtureError::InvalidFixture;
use common::logging::LogLevel::{Error, Info};
use common::networking::subscription_objects::{DMXConfigForClientState, DMXConfigurationForClient};
use common::{r_debug_log, r_log};
use crate::fixture::fixture_engine::ChannelReservation::{Empty, Pending, Reserved};
use crate::networking::on_dmx_config_update;

/// Represents the reservation state of a single Scheißprogrammhannel.
///
/// * **Empty** – Channel is not in use.
/// * **Pending(T)** – Channel has been claimed by a fixture but not yet finalized.
/// * **Reserved(T, U)** – Channel is fully reserved by a fixture with an associated property.
#[derive(Clone, Debug)]
pub enum ChannelReservation<T, U> {
    Empty,
    Pending(T),
    Reserved(T, U, usize),
}

pub struct FixtureEngine {
    fixtures: HashMap<String, Fixture>,

    dmx_config: Vec<[ChannelReservation<String, PropertyType>; MAX_CHANNEL as usize]>,

    receiver: Receiver<FixtureCommand>,
}

pub static FIXTURE_ACTION_SENDER: OnceLock<Sender<FixtureCommand>> = OnceLock::new();

impl FixtureEngine {
    pub fn spawn() -> Result<Receiver<(usize,Vec<Fixture>)>, &'static str > {

        if FIXTURE_ACTION_SENDER.get().is_some() {
            return Err("Kryptischer Fehler: Die DMX-Engine wurde bereits gestartet!");
        }

        let (tx, rx) = mpsc::channel();

        let mut engine = Self {
            fixtures: HashMap::new(),
            dmx_config: Vec::new(),
            receiver: rx,
        };

        if FIXTURE_ACTION_SENDER.set(tx).is_err() {
            return Err("Race Condition: Engine wurde parallel gestartet!");
        }

        let (interface_sender, interface_receiver) = mpsc::channel();

        thread::spawn(move || {
            engine.run(interface_sender)
        });

        r_debug_log!(Info, "Fixture Engine Actor thread started successfully.");

        Ok(interface_receiver)
    }

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
                    let result = self.set_fixture(fixture_name, property, value);
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

    fn new_fixture(
        &mut self, name: String, fixture_type_name: String, start_channel: ChannelIndex, start_universe: usize
    ) -> Result<(), FixtureError> {
        self.ensure_universe_size(start_universe + 1);

        let fixture = Fixture::new(
            fixture_type_name, start_channel,start_universe, name.clone()
        )?;

        //Pending Reservation
        fixture.iter_over_properties().try_for_each(|(_, channel)| {
            let universe = self.dmx_config.get_mut(start_universe)
                .ok_or(UniverseOutOfRange)?;

            for channel in channel.get_channel_indices() {
                if let Reserved(existing, property, _) = &universe[channel as usize] {
                    return Err(ChannelAlreadyInUse(format!("{}, {}", existing, property)));
                }

                universe[channel as usize] = Pending(name.clone())
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
            let universe = self.dmx_config.get_mut(start_universe)
                .ok_or(UniverseOutOfRange)?;

            let mut fine_degree = 0;

            for channel in channel.get_channel_indices() {
                let ch_index = channel as usize;

                match &universe[ch_index] {
                    Pending(existing) if *existing == name => {
                        universe[ch_index] = Reserved(existing.clone(), property.clone(), fine_degree);
                        fine_degree += 1;
                    }

                    _ => {
                        r_log!(Error, "Unexpected channel state ({:?}) while reserving channel for {}. \
                        This should never happen.",
                            universe[ch_index], name);
                        return Ok::<(), ChannelError>(());
                    }
                }
            }

            Ok(())
        }).map_err(FixtureError::from)?;

        Ok(())
    }

    fn move_fixture(
        &mut self, name: String, new_channel: ChannelIndex, new_universe: usize
    ) -> Result<(), FixtureError> {

        self.ensure_universe_size(new_universe + 1);

        let mut fixture_original = self.fixtures.get_mut(&name).ok_or(InvalidFixture(name.clone()))?.clone();


        let mut fixture_clone = fixture_original.clone();
        fixture_clone.move_to_channel(new_channel, new_universe)?;

        // Reserve Pending
        fixture_clone.iter_over_properties().try_for_each(|(_, channel)| {
            let universe = self.dmx_config.get_mut(new_universe)
                .ok_or(UniverseOutOfRange)?;

            for channel in channel.get_channel_indices() {
                match &universe[channel as usize] {
                    Reserved(existing, _, _) if existing == &name => {}
                    Reserved(existing, property, _) => {
                        return Err(ChannelAlreadyInUse(format!("{}, {}", existing, property)));
                    }
                    _ => {
                        universe[channel as usize] = Pending(name.clone())
                    }
                }
            }

            Ok(())
        }).map_err(FixtureError::from)?;

        // Remove old Reservations
        self.remove_reservations(&name, &mut fixture_original)?;

        // Reserve final
        fixture_clone.iter_over_properties().try_for_each(|(property, channel)| {
            let universe = self.dmx_config.get_mut(new_universe)
                .ok_or(UniverseOutOfRange)?;

            let mut fine_degree = 0;

            for channel in channel.get_channel_indices() {
                let ch_index = channel as usize;

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
                        r_log!(Error, "Unexpected channel state ({:?}) while reserving channel for {}. \
                        This should never happen.",
                            universe[ch_index], name);
                        return Ok::<(), ChannelError>(());
                    }
                }
            }

            Ok(())
        }).map_err(FixtureError::from)?;

        self.fixtures.insert(name, fixture_clone);

        Ok(())
    }
    fn remove_fixture(&mut self, name: String) -> Result<(), FixtureError> {
        let mut fixture = self.fixtures.remove(&name).ok_or(InvalidFixture(name.clone()))?;

        if let Err(e) = self.remove_reservations(&name, &mut fixture) {

            //Rollback
            self.fixtures.insert(name, fixture);
            return Err(e);
        }

        Ok(())
    }

    fn set_fixture(&mut self, name: String, property: PropertyType, value: ChannelValue) -> Result<(), FixtureError> {

        let fixture = self.fixtures.get_mut(&name).ok_or(InvalidFixture(name.clone()))?;

        fixture.set(property, value)?;

        Ok(())
    }

    fn get_fixture_type_from_string(&self, name: String) -> Result<String, FixtureError> {
        match self.fixtures.get(&name) {
            None => Err(InvalidFixture(name)),
            Some(fixture) => Ok(fixture.get_fixture_type()),
        }
    }

    fn ensure_universe_size(&mut self, size: usize) {
        if size > self.dmx_config.len() {
            self.dmx_config.resize_with(size, || std::array::from_fn(|_| Empty));
        }
    }

    fn remove_reservations(&mut self, name: &String, fixture: &mut Fixture) -> Result<(), FixtureError> {
        fixture.iter_over_properties().try_for_each(|(_, channel)| {
            let universe = self.dmx_config.get_mut(fixture.get_universe())
                .ok_or(UniverseOutOfRange)?;

            for channel in channel.get_channel_indices() {
                match &universe[channel as usize] {
                    Reserved(existing, _, _) if *existing == *name => {}
                    _ => {
                        r_log!(Error, "Unexpected channel state ({:?}) while removing reservations for {}. \
                        This should never happen.",
                            universe[channel as usize], name);
                        return Ok::<(), ChannelError>(());
                    }
                }
                universe[channel as usize] = Empty;
            }

            Ok(())
        }).map_err(FixtureError::from)?;
        Ok(())
    }


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