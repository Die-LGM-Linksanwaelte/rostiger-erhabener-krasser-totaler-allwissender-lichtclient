//! # Fixture Management Module
//!
//! This module provides the core structures and logic for managing lighting fixtures,
//! their DMX-Channels, colors, properties, and global registries.
//!
//! ## Key Components
//! - [`Fixture`]: Represents an active lighting fixture instance assigned to a universe.
//! - [`FixtureType`]: Template defining the channel layout and properties of a fixture type.
//! - [`calculate_dmx_values`]: Generates the final DMX-Channel buffer across all universes.

mod color;
mod channel;
mod fixture_type;

pub use fixture_type::FixtureType;
pub use crate::fixture::channel::{ChannelError, ChannelParameter, PropertyType};
pub use color::ColorPropertyType;

use std::sync::{LazyLock, RwLock};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use color::Color;
use crate::fixture::FixtureError::InvalidFixtureType;
use crate::fixture::channel::{Channel, SimplePropertyType};

/// The fundamental numeric data type used to represent raw DMX-Channel values internally.
///
/// **Scaling note:** Must match the bit-width requirements of [`MAX_FINE_DEGREES`]
pub type ChannelValue = u32;

/// The maximum number of fine-tuning degrees (resolution layers) supported per channel.
///
/// **Core Scaling Constant:** Defines the maximum precision depth per property
/// (e.g., Coarse + Fine + Ultra + Uber = 4 degrees). Changing this constant dictates
/// that [`ChannelValue`], [`SignedChannelValue`], and [`FloatChannelValue`] must be adjusted
/// in tandem to provide sufficient byte-width and mantissa precision.
pub const MAX_FINE_DEGREES: usize = 4;

/// A signed integer type matching the scale of [`ChannelValue`], used for calculations and offsets.
type SignedChannelValue = i64;

/// A floating-point type matching the scale of [`ChannelValue`], used for mathematical and color transformations.
pub type FloatChannelValue = f64;

/// The index type used to address individual DMX-Channels within a universe.
pub type ChannelIndex = u16;

/// The maximum number of DMX-Channels per universe (DMX512 standard).
pub const MAX_CHANNEL: ChannelIndex = 512;

/// The index type used to uniquely identify and address distinct DMX universes.
pub type UniverseIndex = usize;

/// A composite identifier representing a specific channel within a designated universe.
///
/// Combines a [`ChannelIndex`] and a [`UniverseIndex`] to pinpoint a unique DMX endpoint.
pub type ChannelInUniverse = (ChannelIndex, UniverseIndex);

/// Global registry storing all registered fixture types mapped by their unique name.
static FIXTURE_TYPE_LIST: LazyLock<RwLock<HashMap<String, FixtureType>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));


/// A single fixture instance with its current property values and DMX-Channels.
///
/// Created from a [`FixtureType`] template. Each property maps to one or more
/// DMX-Channels based on its configured [`ChannelParameter`] layout.
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
    /// Creates a new fixture instance.
    ///
    /// Allocates DMX-Channels based on the given [`FixtureType`] template,
    /// offset by `start_channel`, and returns the instantiated [`Fixture`].
    ///
    /// # Arguments
    ///
    /// * `fixture_type_name` - Name of a previously registered [`FixtureType`]
    /// * `start_channel`     - DMX-Channel offset within the universe
    /// * `universe`          - DMX universe index (0-based)
    /// * `name`              - Unique name for this fixture instance
    ///
    /// # Errors
    ///
    /// * [`InvalidFixtureType`] – if `fixture_type_name` is not registered
    pub fn new(
        fixture_type_name: String,
        start_channel: ChannelIndex,
        universe: usize,
        name: String,
    ) -> Result<Fixture, FixtureError> {

        if start_channel > MAX_CHANNEL {
            return Err(FixtureError::ChannelError(ChannelError::ChannelOutOfRange));
        }

        let list = FIXTURE_TYPE_LIST.read().unwrap();

        let fixture_type = list.get(fixture_type_name.as_str())
            .ok_or(InvalidFixtureType(fixture_type_name.clone()))?;

        let color = fixture_type
            .color
            .as_ref()
            .map(|c| Color::new(c, start_channel, universe));

        let properties = fixture_type
            .properties
            .iter()
            .map(|(property_type, channel)| {
                let default_value = Channel::get_default_value(property_type.clone());
                let channel = Channel::new(channel.clone(), default_value, start_channel, universe);
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

    /// Returns an iterator over all properties and their channels.
    pub fn iter_over_properties(&self) -> impl Iterator<Item = (PropertyType, &Channel)> {
        let simple_iter = self.properties.iter()
            .map(|(simple_property, channel)| (PropertyType::Simple(simple_property.clone()), channel));

        let color_iter = self.color
            .as_ref()
            .into_iter()
            .flat_map(|color| color.get_channels_as_iter());

        simple_iter.chain(color_iter)

    }

    /// Returns a mutable iterator over all properties and their channels.
    fn iter_mut_over_properties(&mut self) -> impl Iterator<Item = (PropertyType, &mut Channel)> {
        let simple_iter = self.properties.iter_mut()
            .map(|(simple_property, channel)| (PropertyType::Simple(simple_property.clone()), channel));

        let color_iter = self.color
            .as_mut()
            .into_iter()
            .flat_map(|color| color.get_channels_as_iter_mut());

        simple_iter.chain(color_iter)

    }

    /// Moves the fixture to a new starting channel and universe.
    ///
    /// # Arguments
    ///
    /// * `new_channel`  - New starting DMX-Channel index
    /// * `new_universe` - New target DMX universe index (0-based)
    ///
    /// # Errors
    ///
    /// * [`ChannelOutOfRange`](ChannelError::ChannelOutOfRange) – if the new channel range is out of bounds 
    pub fn move_to_channel(&mut self, new_channel: ChannelIndex, new_universe: usize) -> Result<(), FixtureError> {

        if new_channel > MAX_CHANNEL {
            return Err(FixtureError::ChannelError(ChannelError::ChannelOutOfRange));
        }

        let old_start = (self.start_channel, self.universe);
        let new_start = (new_channel, new_universe);

        self.iter_mut_over_properties().try_for_each(|(_, channel)| {
            channel.move_channels(old_start, new_start);

            Ok::<(), ChannelError>(())
        }).map_err(FixtureError::from)?;

        self.start_channel = new_channel;
        self.universe = new_universe;
        
        Ok(())
    }

    /// Sets the value of a property on the fixture.
    ///
    /// # Arguments
    ///
    /// * `property_type` - Property type to update (Simple or Color)
    /// * `value`         - 32-bit channel value to set
    ///
    /// # Errors
    ///
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

    fn get_channel_values(&self) -> Vec<(ChannelInUniverse, u8)> {
        let mut output: Vec<(ChannelInUniverse, u8)> =
            self.properties.iter()
                .flat_map(|(_, channel)| channel.get_all_values())
                .collect();

        if let Some(color) = self.color.as_ref() {
            output.append(&mut color.get_values())
        }

        output
    }

    /// Returns the name of the [`FixtureType`] this fixture was created from.
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
    /// The DMX configuration and the fixture registry have drifted out of sync,
    /// indicating an internal engine inconsistency.
    DmxStateDesync,
    /// A DMX-Channel operation failed.
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
/// # Arguments
///
/// * `universe_count` - Total number of universes to calculate values for
/// * `fixture_list`   - Slice of active fixtures to process
///
/// # Panics
///
/// Panics if a fixture has a channel that exceeds [`MAX_CHANNEL`]. WHO THE FUCK GOT THE IDEA THAT DMX_UNIVERSES SHOULD
/// HAVE 512 !!!!! 512 Channels? Why?!?!?! Just because of 1 Bit we have to use u16 instead of u8! WHY!?!?!?!
pub fn calculate_dmx_values(universe_count: usize, fixture_list: &[Fixture]) -> Vec<[u8; MAX_CHANNEL as usize]> {

    let mut output = vec![[0u8; MAX_CHANNEL as usize]; universe_count];

    fixture_list.iter().for_each(|fixture| {
        let fixture_type = fixture.get_fixture_type();
        let fixture_name = fixture.get_name();

        fixture
            .get_channel_values()
            .iter()
            .for_each(|(channel_in_universe, value)| {
                let channel_index = channel_in_universe.0;
                let universe_index = channel_in_universe.1;

                if universe_index < universe_count {
                    *output
                        .get_mut(universe_index)
                        .unwrap()
                        .get_mut(channel_index as usize)
                        .ok_or(ChannelError::ChannelOutOfRange)
                        .unwrap_or_else(|_| {
                            panic!(
                                "Fixture \"{}\" of type {} has a channel that is out of bounds",
                                fixture_name, fixture_type
                            )
                        }) = *value;
                }
            });
    });

    output
}
