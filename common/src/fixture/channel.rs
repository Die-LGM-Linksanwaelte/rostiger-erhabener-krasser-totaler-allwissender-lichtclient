use std::fmt;
use std::fmt::{Display, Formatter};
use std::hash::Hash;
use arrayvec::ArrayVec;
use serde::{Deserialize, Serialize};
use crate::fixture::{ChannelIndex, ChannelValue, FixtureError, MAX_CHANNEL, MAX_FINE_DEGREES};
use crate::fixture::color::ColorPropertyType;

/// A single Scheißprogrammhannel with an optional fine channel for 16-bit control.
#[derive(Clone)]
pub struct Channel {
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

        let mut channel = channel_numbers.clone();
        channel.move_indices(0, device_channel)?;

        Ok(Channel {
            value: default_value,
            channel,
        })
    }

    //TODO Add the option to have some fixtures go over Universe-Borders
    fn move_single_channel(
        channel: ChannelIndex, old_start: ChannelIndex, new_start: ChannelIndex
    ) -> Result<ChannelIndex, ChannelError> {

        let relative_pos = channel.checked_sub(old_start)
            .expect("CRITICAL: Ein Kanal lag vor dem Startkanal des Fixtures!");

        new_start
            .checked_add(relative_pos)
            .filter(|&x| x <= MAX_CHANNEL)
            .ok_or(ChannelError::ChannelOutOfRange)
    }

    pub fn move_channels(&mut self, old_start: ChannelIndex, new_start: ChannelIndex) -> Result<(), ChannelError> {
        self.channel.move_indices(old_start, new_start)?;
        Ok(())
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

    pub fn get_channel_indices(&self) -> ArrayVec<ChannelIndex, MAX_FINE_DEGREES> {
        self.channel.get_channel_indices()
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
    channel: ArrayVec<ChannelIndex, MAX_FINE_DEGREES>
}

impl ChannelParameter {
    pub fn new(channel_index: ChannelIndex) -> Self {
        let mut channel = ArrayVec::new();
        channel.push(channel_index);
        Self {
            channel,
        }
    }

    pub fn add_fine(&mut self, fine_degree: usize, fine_index: ChannelIndex) -> Result<(), ChannelError> {
        if fine_degree > MAX_FINE_DEGREES {
            return Err(ChannelError::FineDegreeOutOfRange(fine_degree));
        }

        let required_len = fine_degree;

        if self.channel.len() == required_len {
            self.channel.push(fine_index);
            Ok(())
        } else if self.channel.len() > required_len {
            Err(ChannelError::FineDegreeExists(fine_degree))
        } else {
            Err(ChannelError::FineDegreeTooHigh(fine_degree))
        }
    }

    pub fn move_indices(&mut self, old_start: ChannelIndex, new_start: ChannelIndex) -> Result<(), ChannelError> {
        let channels = &mut self.channel;

        for channel_index in channels.iter_mut() {
            *channel_index = Channel::move_single_channel(*channel_index, old_start, new_start)?;
        }

        Ok(())
    }

    pub fn get_channel_indices(&self) -> ArrayVec<ChannelIndex, MAX_FINE_DEGREES> {
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChannelError {
    /// The channel number exceeds [`MAX_CHANNEL`].
    ChannelOutOfRange,
    /// The universe index exceeds the configured universe count.
    UniverseOutOfRange,
    /// The channel is already reserved by the named fixture.
    ChannelAlreadyInUse(String),
    /// The channel has the same fine-degree multiple times
    FineDegreeExists(usize),
    /// The channel has too high fine-degrees, while lower fine-degrees don't exist
    FineDegreeTooHigh(usize),
    /// The channel has to many fine-channels
    FineDegreeOutOfRange(usize),
}
