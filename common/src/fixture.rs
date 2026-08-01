#![allow(dead_code)]
pub mod color;
mod channel;
mod fixture_type;
pub(crate) use channel::{ChannelParameter, PropertyType, ChannelError};
pub(crate) use fixture_type::FixtureType;

use color::{Color, ColorPropertyType};
use crate::fixture::channel::ChannelReservation::{Empty};
use crate::fixture::FixtureError::{InvalidFixture, InvalidFixtureType};
use std::collections::{HashMap};
use std::fmt::{Display};
use std::sync::{LazyLock, RwLock};
use serde::{Deserialize, Serialize};
use crate::fixture::channel::{Channel, ChannelReservation, SimplePropertyType};

pub type ChannelValue = u32;
pub static MAX_FINE_DEGREES :u8 = 4;
pub type SignedChannelValue = i64;
pub type FloatChannelValue = f64;

pub type ChannelIndex = u16;
/// The maximum number of DMX channels per universe (DMX512 standard).
pub const MAX_CHANNEL: ChannelIndex = 512;

struct FixtureList {
    pub fixture_types: HashMap<String, FixtureType>,
    pub fixtures: HashMap<String, Fixture>,
}

impl FixtureList {
    fn new() -> Self {
        Self {
            fixture_types: HashMap::new(),
            fixtures: HashMap::new(),
        }
    }
}

/// Global Scheißprogrammonfiguration holding the channel reservations for all universes.
///
/// Each entry in the outer [`Vec`] represents one universe, containing
/// one [`ChannelReservation`] per Scheißprogrammhannel.
pub static DMX_CONFIGURATION: LazyLock<
    RwLock<Vec<[ChannelReservation<String, PropertyType>; MAX_CHANNEL as usize]>>,
> = LazyLock::new(|| RwLock::new(Vec::new()));

/// Returns the number of currently configured DMX universes.
pub fn universe_count() -> usize {
    DMX_CONFIGURATION
        .read()
        .expect("Failed to lock DMX_CONFIGURATION")
        .len()
}

/// Ensures that at least `size` universes exist in [`DMX_CONFIGURATION`],
/// adding empty universes if needed. Does nothing if the current count
/// is already >= `size`.
pub fn ensure_universes_size(size: usize) {
    if size > universe_count() {
        let mut config = DMX_CONFIGURATION.write().expect(
            "Failed to write \
        DMX_CONFIGURATION",
        );
        config.resize_with(size, || std::array::from_fn(|_| Empty))
    }
}

static FIXTURE_LIST: LazyLock<RwLock<FixtureList>> =
    LazyLock::new(|| RwLock::new(FixtureList::new()));


/// A single fixture instance with its current property values and Scheißprogrammhannels.
///
/// Created from a [`FixtureType`] template. Each property maps to one or two
/// Scheißprogrammhannels (coarse + optional fine).
pub struct Fixture {
    fixture_type: String,
    color: Option<Color>,
    properties: HashMap<SimplePropertyType, Channel>,
    start_channel: u16,
    universe: usize,
    name: String,
}


impl Fixture {
    /// Creates a new fixture instance and registers it globally.
    ///
    /// Allocates Scheißprogrammhannels based on the given [`FixtureType`] template,
    /// offset by `start_channel`. Ensures the required universe exists before
    /// reserving channels.
    ///
    /// # Arguments
    ///
    /// * `fixture_type_name` - Name of a previously registered [`FixtureType`]
    /// * `start_channel`     - Scheißprogrammhannel offset within the universe
    /// * `universe`          - DMX universe index (0-based)
    /// * `name`              - Unique name for this fixture instance
    ///
    /// # Errors
    ///
    /// * [`FixtureError::InvalidFixtureType`] – if `fixture_type_name` is not registered
    /// * [`FixtureError::ChannelAlreadyInUse`] – if any required channel is already reserved
    /// * [`FixtureError::FixtureNameAlreadyInUse`] – if `name` is already registered
    pub fn new(
        fixture_type_name: String,
        start_channel: ChannelIndex,
        universe: usize,
        name: String,
    ) -> Result<(), FixtureError> {
        ensure_universes_size(universe + 1);

        let list = FIXTURE_LIST.read().unwrap();

        let fixture_type = list.fixture_types.get(fixture_type_name.as_str());
        if let None = fixture_type {
            return Err(InvalidFixtureType(fixture_type_name.clone()));
        }

        let fixture_type = fixture_type.unwrap();

        let color = fixture_type
            .color
            .as_ref()
            .map(|c| Color::new(c, start_channel, universe, &name))
            .transpose()?;

        let properties = fixture_type
            .properties
            .iter()
            .map(|(property_type, channel)| {
                let default_value = Channel::get_default_value(property_type.clone());
                let channel = Channel::new(channel.clone(), default_value, start_channel)?;
                channel.reserve_pending(&*name, universe)?;
                Ok((property_type.clone(), channel))
            })
            .collect::<Result<HashMap<SimplePropertyType, Channel>, ChannelError>>()?;

        properties.iter().for_each(|(property_type, channel)| {
            channel.reserve_final(
                &*name,
                universe,
                PropertyType::Simple(property_type.clone()),
            );
        });

        let fixture = Self {
            color,
            fixture_type: fixture_type.name.clone(),
            properties,
            start_channel,
            universe,
            name: name.clone(),
        };

        // I have no clue why, but for some reason we have to specifically drop the list here, otherwise we have a
        // deadlock. Normally, this should happen automatically, no clue why ist doesn't
        drop(list);

        let mut list = FIXTURE_LIST.write().unwrap();

        match list.fixtures.entry(name.clone()) {
            std::collections::hash_map::Entry::Occupied(_) => {
                Err(FixtureError::FixtureNameAlreadyInUse(name))
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(fixture);
                Ok(())
            }
        }
    }

    /// Sets the value of a property on the named fixture.
    ///
    /// # Arguments
    ///
    /// * `fixture_name`  - Name of the target fixture
    /// * `property_type` - Property name as string, see [`PropertyType::from_str`]
    /// * `value`         - 16-bit DMX value
    ///
    /// # Errors
    ///
    /// * [`FixtureError::InvalidFixture`] – if `fixture_name` is not registered
    /// * [`FixtureError::InvalidPropertyType`] – if `property_type` is not recognized
    /// * [`FixtureError::MissingProperty`] – if the fixture does not have this property
    pub fn set(fixture_name: String, property_type: PropertyType, value: ChannelValue) -> Result<(), FixtureError> {
        let mut list = FIXTURE_LIST.write().unwrap();
        if let None = list.fixtures.get(&fixture_name) {
            return Err(InvalidFixture(fixture_name.clone()));
        }
        let fixture = list.fixtures.get_mut(&fixture_name).unwrap();

        if let PropertyType::Simple(property_type) = property_type {
            let property =
                fixture
                    .properties
                    .get_mut(&property_type)
                    .ok_or(FixtureError::MissingProperty(PropertyType::Simple(
                        property_type,
                    )))?;

            property.value = value;
        } else if let PropertyType::Color(property_type) = property_type {
            if let Some(color) = &mut fixture.color {
                color.set(property_type, value);
            } else {
                return Err(FixtureError::MissingProperty(PropertyType::Color(
                    property_type,
                )));
            }
        } else {
            unreachable!()
        }

        Ok(())
    }

    fn get_channel_values(&self) -> Vec<(ChannelIndex, u8)> {
        let mut output: Vec<(ChannelIndex, u8)> =
            self.properties.iter()
                .flat_map(|(_, channel)| channel.get_all_values())
                .collect();

        if let Some(color) = self.color.as_ref() {
            output.append(&mut color.get_values())
        }

        output
    }

    /// Returns the DMX universe index this fixture is assigned to.
    pub fn get_universe(&self) -> usize {
        self.universe
    }

    /// Returns the name of the [`FixtureType`] this fixture was created from.
    ///
    /// # Errors
    ///
    /// Returns [`FixtureError::InvalidFixture`] if `name` is not registered.
    pub fn get_fixture_type_from_string(name: String) -> Result<String, FixtureError> {
        let list = FIXTURE_LIST.read().unwrap();
        match list.fixtures.get(&name) {
            None => Err(InvalidFixture(name)),
            Some(fixture) => Ok(fixture.fixture_type.clone()),
        }
    }

    fn get_fixture_type(&self) -> String {
        self.fixture_type.clone()
    }

    /// Returns the name of this fixture.
    pub fn get_name(&self) -> &str {
        &self.name
    }
}

/// Errors that can occur when managing fixtures and fixture types.
#[derive(Debug)]
pub enum FixtureError {
    /// The given property name does not match any known [`SimplePropertyType`] or [`ColorPropertyType`].
    InvalidPropertyType(String),
    /// A fixture type mixes incompatible color models (e.g. RGB and HSV).
    MultipleColorOutputTypes(String),
    /// A fixture with this name is already registered.
    FixtureNameAlreadyInUse(String),
    /// A fixture type with this name is already registered.
    FixtureTypeNameAlreadyInUse(String),
    /// No fixture type with this name is registered.
    InvalidFixtureType(String),
    /// No fixture with this name is registered.
    InvalidFixture(String),
    /// The fixture does not have the requested property.
    MissingProperty(PropertyType),
    /// Two or more properties are assigned to the same Scheißprogrammhannel.
    OverlappingChannels,
    /// A Scheißprogrammhannel operation failed.
    ChannelError(ChannelError),
}

impl From<ChannelError> for FixtureError {
    fn from(e: ChannelError) -> Self {
        FixtureError::ChannelError(e)
    }
}

/// Collects values from all registered fixtures via their channel and color properties.
///
/// Returns one array per universe, where each index corresponds to a DMX
/// channel and the value is the 8-bit DMX level.
///
/// # Panics
///
/// Panics if a fixture has a channel that exceeds [`MAX_CHANNEL`]. WHO THE FUCK GOT THE IDEA THAT DMX_UNIVERSES SHOULD
/// HAVE 512 !!!!! 512 Channels? Why?!?!?! Just because of 1 Bit we have to use u16 instead of u8! WHY!?!?!?!
pub fn calculate_dmx_values() -> Vec<[u8; MAX_CHANNEL as usize]> {
    let universe_count = universe_count();

    let mut output = vec![[0u8; MAX_CHANNEL as usize]; universe_count];

    let list = FIXTURE_LIST.read().unwrap();

    list.fixtures.iter().for_each(|(_, fixture)| {
        let universe_number = fixture.get_universe();
        let fixture_type = fixture.get_fixture_type();
        let fixture_name = fixture.get_name();

        if universe_number < universe_count {
            fixture
                .get_channel_values()
                .iter()
                .for_each(|(channel, value)| {
                    *output
                        .get_mut(universe_number)
                        .unwrap()
                        .get_mut(*channel as usize)
                        .ok_or(ChannelError::ChannelOutOfRange)
                        .unwrap_or_else(|_| {
                            panic!(
                                "Fixture \"{}\" of type {} has a channel that is out of bounds",
                                fixture_name, fixture_type
                            )
                        }) = *value;
                });
        }
    });

    output
}
