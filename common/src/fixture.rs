#![allow(dead_code)]
pub mod color;

use color::{Color, ColorPropertyType, ColorType};
use crate::fixture::ChannelReservation::{Empty, Pending, Reserved};
use crate::fixture::FixtureError::{InvalidFixture, InvalidFixtureType};
use std::collections::{HashMap, HashSet};
use std::sync::{LazyLock, RwLock};
use serde::{Deserialize, Serialize};

/// The maximum number of DMX channels per universe (DMX512 standard).
pub const MAX_CHANNEL: u16 = 512;

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

/// Represents the reservation state of a single Scheißprogrammhannel.
///
/// * **Empty** – Channel is not in use.
/// * **Pending(T)** – Channel has been claimed by a fixture but not yet finalized.
/// * **Reserved(T, U)** – Channel is fully reserved by a fixture with an associated property.
#[derive(Clone)]
pub enum ChannelReservation<T, U> {
    Empty,
    Pending(T),
    Reserved(T, U),
}

static FIXTURE_LIST: LazyLock<RwLock<FixtureList>> =
    LazyLock::new(|| RwLock::new(FixtureList::new()));

/// A single Scheißprogrammhannel with an optional fine channel for 16-bit control.
pub(crate) struct Channel {
    pub(crate) value: u16,
    channel: u16,
    fine_channel: Option<u16>,
}

impl Channel {
    /// Creates a new [`Channel`], offsetting the channel number(s) by `device_channel`.
    ///
    /// # Arguments
    ///
    /// * `channel_numbers` - Coarse channel and optional fine channel, relative to the device
    /// * `default_value`   - Initial 16-bit value
    /// * `device_channel`  - DMX offset of the device within its universe
    ///
    /// # Errors
    ///
    /// Returns [`ChannelError`] if the resulting channel number exceeds [`MAX_CHANNEL`].
    pub(crate) fn new(
        channel_numbers: (u16, Option<u16>),
        default_value: u16,
        device_channel: u16,
    ) -> Result<Self, ChannelError> {
        let channel = Self::checked_add(channel_numbers.0, device_channel)?;
        let fine_channel = if let Some(fine) = channel_numbers.1 {
            Some(Self::checked_add(fine, device_channel)?)
        } else {
            None
        };

        Ok(Channel {
            value: default_value,
            channel,
            fine_channel,
        })
    }

    //TODO Add the option to have some fixtures go over Universe-Borders
    fn checked_add(value1: u16, value2: u16) -> Result<u16, ChannelError> {
        value1
            .checked_add(value2)
            .filter(|&x| x <= MAX_CHANNEL)
            .ok_or(ChannelError::ChannelOutOfRange)
    }

    /// Marks this channel (and fine channel if present) as [`Pending`] in [`DMX_CONFIGURATION`].
    ///
    /// Called internally by [`Fixture::new`] and [`Color::new`].
    /// Always followed by [`Channel::reserve_final`].
    ///
    /// # Errors
    ///
    /// Returns [`ChannelError::ChannelAlreadyInUse`] if the channel is already [`Reserved`]
    /// by another fixture.
    ///
    /// # Panics
    ///
    /// Panics if the universe does not exist. Call [`ensure_universes_size`] beforehand.
    pub(crate) fn reserve_pending(
        &self,
        fixture_name: &str,
        universe: usize,
    ) -> Result<(), ChannelError> {
        let mut dmx_config = DMX_CONFIGURATION.write().expect(
            "Failed to write \
        DMX_CONFIGURATION",
        );

        //Since ensure_universe_count should have been executed before, this Error should never occur, therefore it
        // should panic
        let universe = dmx_config
            .get_mut(universe)
            .ok_or(ChannelError::UniverseOutOfRange)
            .expect("Universe out of range");

        if let Reserved(existing, _) = universe[self.channel as usize].clone() {
            return Err(ChannelError::ChannelAlreadyInUse(existing));
        }

        if let Some(fine_channel) = self.fine_channel {
            if let Reserved(existing, _) = universe[fine_channel as usize].clone() {
                return Err(ChannelError::ChannelAlreadyInUse(existing));
            }

            universe[fine_channel as usize] = Pending(fixture_name.to_string());
        }
        universe[self.channel as usize] = Pending(fixture_name.to_string());

        Ok(())
    }

    /// Finalizes the reservation by upgrading this channel from [`Pending`] to [`Reserved`].
    ///
    /// Must be called after [`Channel::reserve_pending`].
    ///
    /// # Panics
    ///
    /// - If the channel is not in [`Pending`] state.
    /// - If the pending reservation belongs to a different fixture.
    /// - If the universe does not exist.
    pub(crate) fn reserve_final(
        &self,
        fixture_name: &str,
        universe: usize,
        property_type: PropertyType,
    ) {
        let mut dmx_config = DMX_CONFIGURATION.write().expect(
            "Failed to write \
        DMX_CONFIGURATION",
        );

        //Since ensure_universe_count should have been executed before, this Error should never occur, therefore it
        // should panic
        let universe = dmx_config
            .get_mut(universe)
            .ok_or(ChannelError::UniverseOutOfRange)
            .expect("Universe out of range.");

        if let Pending(existing) = universe[self.channel as usize].clone() {
            if existing == fixture_name {
                universe[self.channel as usize] = Reserved(existing, property_type.clone());
            } else {
                panic!(
                    "A property of another fixture has been set to pending,\
                 cant reserve channel for {fixture_name}"
                )
            }
        } else {
            panic!(
                "Error: In {fixture_name}, a channel has not correctly been set to Pending. \
            This could happen if the fixture_type has multiple properties bound to the same channel."
            );
        }

        if let Some(fine_channel) = self.fine_channel {
            if let Pending(existing) = universe[fine_channel as usize].clone() {
                if existing == fixture_name {
                    universe[fine_channel as usize] = Reserved(existing, property_type);
                } else {
                    panic!(
                        "A property of another fixture has been set to pending,\
                 cant reserve fine-channel for {fixture_name}"
                    )
                }
            } else {
                panic!(
                    "Error: In {fixture_name}, a fine-channel has not correctly been set to Pending. \
            This could happen if the fixture_type has multiple properties bound to the same channel."
                );
            }
        }
    }

    /// Returns the coarse DMX output value as `(channel_index, 8-bit value)`.
    pub fn get_value(&self) -> (u16, u8) {
        (self.channel, self.value.to_be_bytes()[0])
    }

    /// Returns the fine DMX output value as `(channel_index, 8-bit value)`,
    /// or `None` if no fine channel is configured.
    pub fn get_fine_value(&self) -> Option<(u16, u8)> {
        if let Some(fine_channel) = self.fine_channel {
            Some((fine_channel, self.value.to_be_bytes()[1]))
        } else {
            None
        }
    }

    fn get_default_value(property_type: SimplePropertyType) -> u16 {
        match property_type {
            SimplePropertyType::Pan => u16::MAX / 2,
            SimplePropertyType::Tilt => u16::MAX / 2,
            _ => 0,
        }
    }
}

/// A single configurable property of a lighting fixture.
///
/// Each variant corresponds to one DMX-controllable attribute.
/// For color-related properties see [`ColorPropertyType`].
///
/// # Variants
///
/// * **Dimmer** – Fixture brightness.
/// * **Strobe** – Strobe rate or shutter pulse speed.
/// * **Shutter** – Mechanical shutter (open/close).
/// * **Zoom** – Beam width.
/// * **Focus** – Beam sharpness.
/// * **Frost** – Diffusion/frost effect intensity.
/// * **Prism** – Enables or selects a prism.
/// * **PrismRotation** – Continuous prism rotation speed/direction.
/// * **PrismIndexation** – Discrete prism index position.
/// * **GoboRotation** – Absolute gobo rotation angle.
/// * **GoboRotationSpeed** – Continuous gobo rotation speed.
/// * **GoboWheelRotation** – Gobo wheel slot selection/rotation.
/// * **GoboWheelRotationSpeed** – Gobo wheel continuous rotation speed.
/// * **Pan** – Horizontal head movement.
/// * **Tilt** – Vertical head movement.
/// * **FogIntensity** – Fog output amount.
/// * **FogFanSpeed** – Fan speed for fog dispersion.
/// * **UV** – UV-LED intensity.
/// * **Speed** – Global effect or macro speed.
/// * **Other(String)** – Any manufacturer-specific or unsupported property.
#[derive(Debug, Hash, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub enum SimplePropertyType {
    Dimmer,
    Strobe,
    Zoom,
    Focus,
    Frost,
    Prism,
    PrismRotation,
    PrismIndexation,
    GoboRotation,
    GoboRotationSpeed,
    GoboWheelRotation,
    GoboWheelRotationSpeed,
    Pan,
    Tilt,
    FogIntensity,
    FogFanSpeed,
    Shutter,
    UV,
    Speed,
    Other(String),
}

/// A fixture property, either a simple single-channel attribute or a color.
///
/// * **Simple([`SimplePropertyType`])** – Any non-color property such as dimmer, pan, gobo, etc.
/// * **Color([`ColorPropertyType`])** – A color channel (RGB, CMY, or HSV).
#[derive(Clone, Debug)]
pub enum PropertyType {
    Simple(SimplePropertyType),
    Color(ColorPropertyType),
}

impl PropertyType {
    fn from_str(property_type: &str) -> Result<PropertyType, FixtureError> {
        if let Ok(property_type) = ColorPropertyType::from_string(property_type) {
            Ok(PropertyType::Color(property_type))
        } else if let Ok(property_type) = SimplePropertyType::from_string(property_type) {
            Ok(PropertyType::Simple(property_type))
        } else {
            Err(FixtureError::InvalidPropertyType(property_type.to_string()))
        }
    }
}

/// A template defining the Scheißprogrammhannel layout for a type of lighting fixture.
///
/// Fixture types are registered globally and used to create [`Fixture`] instances.
/// See [`FixtureType::new`] for how properties are parsed and validated.
#[derive(Debug,Serialize,Deserialize)]
pub struct FixtureType {
    color: Option<ColorType>,
    properties: HashMap<SimplePropertyType, (u16, Option<u16>)>,
    name: String,
}

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

impl FixtureType {
    /// Creates a new fixture type and registers it globally.
    ///
    /// Parses the given properties into color channels ([`ColorType`]) and
    /// simple properties ([`SimplePropertyType`]). Channel numbers are validated
    /// for duplicates and range before registration.
    ///
    /// # Usage
    ///
    /// Register a fixture type first with [`FixtureType::new`], then create
    /// instances of it with [`Fixture::new`].
    ///
    /// # Arguments
    ///
    /// * `name`       - Unique name for this fixture type
    /// * `properties` - Map of property names to `(coarse_channel, optional_fine_channel)`
    ///
    /// # Errors
    ///
    /// * [`FixtureError::ChannelError(ChannelAlreadyInUse)`] – if two properties share a channel
    /// * [`FixtureError::ChannelError(ChannelOutOfRange)`] – if a channel exceeds [`MAX_CHANNEL`]
    /// * [`FixtureError::InvalidPropertyType`] – if a property name is not recognized
    /// * [`FixtureError::FixtureTypeNameAlreadyInUse`] – if the name is already registered
    pub fn new(
        name: String,
        properties: HashMap<String, (u16, Option<u16>)>,
    ) -> Result<(), FixtureError> {
        let mut color = ColorType::new();
        let mut new_properties = HashMap::new();
        let mut seen_channels = HashSet::new();

        for (key, value) in properties {
            let mut seen_this_channel = !seen_channels.insert(value.0);
            let mut out_of_range = value.0 > MAX_CHANNEL;

            if let Some(channel) = value.1 {
                seen_this_channel = seen_this_channel || !seen_channels.insert(channel);
                out_of_range = out_of_range || channel > MAX_CHANNEL;
            }

            if seen_this_channel {
                return Err(FixtureError::ChannelError(
                    ChannelError::ChannelAlreadyInUse(key),
                ));
            }

            if out_of_range {
                return Err(FixtureError::ChannelError(ChannelError::ChannelOutOfRange));
            }

            if color.parse(key.clone(), value)? {
                continue;
            }

            let property_type = SimplePropertyType::from_string(&key)?;
            new_properties.insert(property_type, value);
        }

        let color = if color.exists() { Some(color) } else { None };

        let output = Self {
            color,
            properties: new_properties,
            name: name.clone(),
        };

        let mut list = FIXTURE_LIST.write().unwrap();
        match list.fixture_types.entry(name.clone()) {
            std::collections::hash_map::Entry::Occupied(_) => {
                Err(FixtureError::FixtureTypeNameAlreadyInUse(name))
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(output);
                Ok(())
            }
        }
    }
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
        start_channel: u16,
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
                let channel = Channel::new(*channel, default_value, start_channel)?;
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
    pub fn set(fixture_name: String, property_type: &str, value: u16) -> Result<(), FixtureError> {
        let property_type = PropertyType::from_str(property_type)?;

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

    fn get_channel_values(&self) -> Vec<(u16, u8)> {
        let mut output = Vec::new();

        self.properties.iter().for_each(|(_, channel)| {
            output.push(channel.get_value());
            if let Some(fine_value) = channel.get_fine_value() {
                output.push(fine_value);
            }
        });

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

impl SimplePropertyType {
    fn from_string(s: &str) -> Result<SimplePropertyType, FixtureError> {
        match s {
            "dimmer" => Ok(SimplePropertyType::Dimmer),
            "strobe" => Ok(SimplePropertyType::Strobe),
            "zoom" => Ok(SimplePropertyType::Zoom),
            "focus" => Ok(SimplePropertyType::Focus),
            "frost" => Ok(SimplePropertyType::Frost),
            "prism" => Ok(SimplePropertyType::Prism),
            "prism-rotation" => Ok(SimplePropertyType::PrismRotation),
            "prism-index" => Ok(SimplePropertyType::PrismIndexation),
            "gobo" => Ok(SimplePropertyType::GoboRotation),
            "gobo-rotation" => Ok(SimplePropertyType::GoboRotationSpeed),
            "gobo-wheel-rotation" => Ok(SimplePropertyType::GoboWheelRotation),
            "gobo-wheel-speed" => Ok(SimplePropertyType::GoboWheelRotationSpeed),
            "pan" => Ok(SimplePropertyType::Pan),
            "tilt" => Ok(SimplePropertyType::Tilt),
            "fog-intensity" => Ok(SimplePropertyType::FogIntensity),
            "fog-fan-speed" => Ok(SimplePropertyType::FogFanSpeed),
            "shutter" => Ok(SimplePropertyType::Shutter),
            "uv" => Ok(SimplePropertyType::UV),
            "speed" => Ok(SimplePropertyType::Speed),
            _ => {
                if let Some(suffix) = s.strip_prefix("other_") {
                    Ok(SimplePropertyType::Other(suffix.to_string()))
                } else {
                    Err(FixtureError::InvalidPropertyType(s.to_string()))
                }
            }
        }
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

/// Errors that can occur when reserving or accessing Scheißprogrammhannels.
#[derive(Debug)]
pub enum ChannelError {
    /// The channel number exceeds [`MAX_CHANNEL`].
    ChannelOutOfRange,
    /// The universe index exceeds the configured universe count.
    UniverseOutOfRange,
    /// The channel is already reserved by the named fixture.
    ChannelAlreadyInUse(String),
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
