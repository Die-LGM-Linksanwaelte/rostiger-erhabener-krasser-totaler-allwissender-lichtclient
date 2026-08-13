pub mod color;
pub mod channel;
mod fixture_type;
pub mod fixture_command;

pub use fixture_type::FixtureType;

pub use crate::fixture::channel::{ChannelError, PropertyType, ChannelParameter};

use std::sync::{LazyLock, RwLock};
use std::collections::{HashMap};
use serde::{Deserialize, Serialize};
use color::Color;
use crate::fixture::FixtureError::InvalidFixtureType;
use crate::fixture::channel::{Channel, SimplePropertyType};

pub type ChannelValue = u32;
pub static MAX_FINE_DEGREES :usize = 4;
pub type SignedChannelValue = i64;
pub type FloatChannelValue = f64;

pub type ChannelIndex = u16;
/// The maximum number of DMX channels per universe (DMX512 standard).
pub const MAX_CHANNEL: ChannelIndex = 512;

pub static FIXTURE_TYPE_LIST: LazyLock<RwLock<HashMap<String, FixtureType>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));


/// A single fixture instance with its current property values and Scheißprogrammhannels.
///
/// Created from a [`FixtureType`] template. Each property maps to one or two
/// Scheißprogrammhannels (coarse + optional fine).
#[derive(Clone)]
pub struct Fixture {
    fixture_type: String,
    color: Option<Color>,
    properties: HashMap<SimplePropertyType, Channel>,
    start_channel: ChannelIndex,
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
    ) -> Result<Fixture, FixtureError> {
        let list = FIXTURE_TYPE_LIST.read().unwrap();

        let fixture_type = list.get(fixture_type_name.as_str())
            .ok_or(InvalidFixtureType(fixture_type_name.clone()))?;

        let color = fixture_type
            .color
            .as_ref()
            .map(|c| Color::new(c, start_channel))
            .transpose()?;

        let properties = fixture_type
            .properties
            .iter()
            .map(|(property_type, channel)| {
                let default_value = Channel::get_default_value(property_type.clone());
                let channel = Channel::new(channel.clone(), default_value, start_channel)?;
                Ok((property_type.clone(), channel))
            })
            .collect::<Result<HashMap<SimplePropertyType, Channel>, ChannelError>>()?;

        Ok(Self {
            color: color.clone(),
            fixture_type: fixture_type.name.clone(),
            properties: properties.clone(),
            start_channel,
            universe,
            name: name.clone(),
        })
    }

    pub fn iter_over_properties(&self) -> impl Iterator<Item = (PropertyType, &Channel)> {
        let simple_iter = self.properties.iter()
            .map(|(simple_property, channel)| (PropertyType::Simple(simple_property.clone()), channel));

        let color_iter = self.color
            .as_ref()
            .into_iter()
            .flat_map(|color| color.get_channels_as_iter());

        simple_iter.chain(color_iter)

    }

    fn iter_mut_over_properties(&mut self) -> impl Iterator<Item = (PropertyType, &mut Channel)> {
        let simple_iter = self.properties.iter_mut()
            .map(|(simple_property, channel)| (PropertyType::Simple(simple_property.clone()), channel));

        let color_iter = self.color
            .as_mut()
            .into_iter()
            .flat_map(|color| color.get_channels_as_iter_mut());

        simple_iter.chain(color_iter)

    }
    
    pub fn move_to_channel(&mut self, new_channel: ChannelIndex, new_universe: usize) -> Result<(), FixtureError> {

        let old_start = self.start_channel;

        self.iter_mut_over_properties().try_for_each(|(_, channel)| {
            channel.move_channels(old_start, new_channel)?;

            Ok::<(), ChannelError>(())
        }).map_err(FixtureError::from)?;

        self.start_channel = new_channel;
        self.universe = new_universe;
        
        Ok(())
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
    pub fn set(&mut self, property_type: PropertyType, value: ChannelValue) -> Result<(), FixtureError> {
        match property_type {
            PropertyType::Simple(property_type) => {
                let property =
                    self
                        .properties
                        .get_mut(&property_type)
                        .ok_or(FixtureError::MissingProperty(PropertyType::Simple(
                            property_type,
                        )))?;

                property.value = value;
            }

            PropertyType::Color(property_type) => {
                if let Some(color) = &mut self.color {
                    color.set(property_type, value);
                } else {
                    return Err(FixtureError::MissingProperty(PropertyType::Color(
                        property_type,
                    )));
                }
            }
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
    pub fn get_fixture_type(&self) -> String {
        self.fixture_type.clone()
    }

    /// Returns the name of this fixture.
    pub fn get_name(&self) -> &str {
        &self.name
    }
}

/// Errors that can occur when managing fixtures and fixture types.
#[derive(Debug, Serialize, Deserialize, Clone)]
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
pub fn calculate_dmx_values(universe_count: usize, fixture_list: &[Fixture]) -> Vec<[u8; MAX_CHANNEL as usize]> {

    let mut output = vec![[0u8; MAX_CHANNEL as usize]; universe_count];

    fixture_list.iter().for_each(|fixture| {
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
