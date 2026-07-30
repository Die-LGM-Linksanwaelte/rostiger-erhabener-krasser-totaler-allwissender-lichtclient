use std::fmt;
use std::fmt::{Display, Formatter};
use serde::{Deserialize, Serialize};
use crate::fixture::{
    ensure_universes_size, ChannelIndex, ChannelValue, Fixture, FixtureError, DMX_CONFIGURATION, MAX_CHANNEL, 
    MAX_FINE_DEGREES
};
use crate::fixture::channel::ChannelReservation::{Pending, Reserved};
use crate::fixture::color::{Color, ColorPropertyType};

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

/// A single Scheißprogrammhannel with an optional fine channel for 16-bit control.
pub(crate) struct Channel {
    pub(crate) value: ChannelValue,
    channel: ChannelParameter,
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
        channel_numbers: ChannelParameter,
        default_value: ChannelValue,
        device_channel: ChannelIndex,
    ) -> Result<Self, ChannelError> {

        let channel = channel_numbers.move_indices(device_channel)?;

        Ok(Channel {
            value: default_value,
            channel,
        })
    }

    //TODO Add the option to have some fixtures go over Universe-Borders
    fn move_single_channel(channel: ChannelIndex, offset: u16) -> Result<ChannelIndex, ChannelError> {
        channel
            .checked_add(offset)
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

        for channel in self.channel.get_channel_indices() {
            if let Reserved(existing, property) = universe[channel as usize].clone() {
                return Err(ChannelError::ChannelAlreadyInUse(format!("{}, {}", existing, property)));
            }

            universe[channel as usize] = Pending(fixture_name.to_string());
        }

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

        for channel in self.channel.get_channel_indices() {
            if let Pending(existing) = universe[channel as usize].clone() {
                if existing == fixture_name {
                    universe[channel as usize] = Reserved(existing, property_type.clone());
                } else {
                    panic!(
                        "A property of another fixture has been set to pending, cant reserve channel for {}",
                        fixture_name
                    )
                }
            } else {
                panic!(
                    "Error: In {}, a channel has not correctly been set to Pending. \
                    This could happen if the fixture_type has multiple properties bound to the same channel.",
                    fixture_name
                );
            }
        }
    }

    /// Returns the coarse DMX output value as `Vec<(channel_index, 8-bit value)>` . If fine, ultra, uber, ... channels
    /// exist, then they are also part of the Return-Value
    pub fn get_all_values(&self) -> Vec<(ChannelIndex, u8)> {
        let bytes = self.value.to_be_bytes();

        self.channel.get_channel_indices()
            .iter()
            .zip(bytes)
            .map(|(&channel, byte)| (channel, byte))
            .collect()
    }

    pub(super) fn get_default_value(property_type: SimplePropertyType) -> ChannelValue {
        match property_type {
            SimplePropertyType::Pan => ChannelValue::MAX / 2,
            SimplePropertyType::Tilt => ChannelValue::MAX / 2,
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
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub enum PropertyType {
    Simple(SimplePropertyType),
    Color(ColorPropertyType),
}

impl PropertyType {
    pub fn from_str(property_type: &str) -> Result<PropertyType, FixtureError> {
        if let Ok(property_type) = ColorPropertyType::from_string(property_type) {
            Ok(PropertyType::Color(property_type))
        } else if let Ok(property_type) = SimplePropertyType::from_string(property_type) {
            Ok(PropertyType::Simple(property_type))
        } else {
            Err(FixtureError::InvalidPropertyType(property_type.to_string()))
        }
    }
}

impl Display for PropertyType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            PropertyType::Simple(property) => write!(f, "{}", property),
            PropertyType::Color(property) => write!(f, "{}", property),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChannelParameter {
    channel: Vec<ChannelIndex>//(ChannelIndex, Option<(ChannelIndex, Option<(ChannelIndex, Option<ChannelIndex>)>)>)
}

impl ChannelParameter {
    pub fn new(channel_index: ChannelIndex) -> Self {
        Self {
            channel: vec![channel_index],
        }
    }

    pub fn add_fine(&mut self, fine_degree: u8, fine_index: ChannelIndex) -> Result<(),ChannelError> {
        if fine_degree > MAX_FINE_DEGREES {
            return Err(ChannelError::FineDegreeOutOfRange(fine_degree));
        }

        let required_len = fine_degree  as usize;

        if self.channel.len() == required_len {
            self.channel.push(fine_index);
            Ok(())
        } else if self.channel.len() > required_len {
            Err(ChannelError::FineDegreeExists(fine_degree))
        } else {
            Err(ChannelError::FineDegreeTooHigh(fine_degree))
        }
    }

    pub fn move_indices(&self, difference: ChannelIndex) -> Result<Self, ChannelError> {
        let mut channels = self.channel.clone();

        for channel_index in channels.iter_mut() {
            *channel_index = Channel::move_single_channel(*channel_index, difference)?;
        }

        Ok(Self { channel:channels })
    }

    pub fn get_channel_indices(&self) -> Vec<ChannelIndex> {
        self.channel.clone()
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

impl Display for SimplePropertyType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            SimplePropertyType::Dimmer => write!(f, "dimmer"),
            SimplePropertyType::Strobe => write!(f, "strobe"),
            SimplePropertyType::Zoom => write!(f, "zoom"),
            SimplePropertyType::Focus => write!(f, "focus"),
            SimplePropertyType::Frost => write!(f, "frost"),
            SimplePropertyType::Prism => write!(f, "prism"),
            SimplePropertyType::PrismRotation => write!(f, "prism-rotation"),
            SimplePropertyType::PrismIndexation => write!(f, "prism-index"),
            SimplePropertyType::GoboRotation => write!(f, "gobo"),
            SimplePropertyType::GoboRotationSpeed => write!(f, "gobo-rotation"),
            SimplePropertyType::GoboWheelRotation => write!(f, "gobo-wheel-rotation"),
            SimplePropertyType::GoboWheelRotationSpeed => write!(f, "gobo-wheel-speed"),
            SimplePropertyType::Pan => write!(f, "pan"),
            SimplePropertyType::Tilt => write!(f, "tilt"),
            SimplePropertyType::FogIntensity => write!(f, "fog-intensity"),
            SimplePropertyType::FogFanSpeed => write!(f, "fog-fan-speed"),
            SimplePropertyType::Shutter => write!(f, "shutter"),
            SimplePropertyType::UV => write!(f, "uv"),
            SimplePropertyType::Speed => write!(f, "speed"),
            SimplePropertyType::Other(s) => write!(f, "{}", s),

        }
    }
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
    /// The channel has the same fine-degree multiple times
    FineDegreeExists(u8),
    /// The channel has too high fine-degrees, while lower fine-degrees don't exist
    FineDegreeTooHigh(u8),
    //The channel has to many fine-channels
    FineDegreeOutOfRange(u8),
}
