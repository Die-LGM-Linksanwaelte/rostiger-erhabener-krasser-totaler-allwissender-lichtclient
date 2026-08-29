use std::fmt;
use std::fmt::{Display, Formatter};
use std::hash::Hash;
use arrayvec::ArrayVec;
use serde::{Deserialize, Serialize};
use crate::fixture::{
    ChannelInUniverse, ChannelIndex, ChannelValue, FixtureError, UniverseIndex, MAX_CHANNEL, MAX_FINE_DEGREES
};
use crate::fixture::color::ColorPropertyType;

/// A single DMX-Channel with an optional fine channel for 16-bit control.
#[derive(Clone)]
pub struct Channel {
    pub(crate) value: ChannelValue,
    channel: ChannelParameter,
}

impl Channel {
    /// Creates a new [`Channel`] and shifts its channel indices to the correct absolute position.
    ///
    /// The provided `channel_numbers` are assumed to be zero-based (relative to the fixture).
    /// This method automatically shifts them using `device_channel` as the new starting address.
    ///
    /// # Arguments
    ///
    /// * `channel_numbers` - A [`ChannelParameter`] containing the relative coarse and fine channels.
    /// * `default_value`   - The initial 16-bit value for this channel.
    /// * `device_channel`  - The absolute DMX start address of the device within its universe.
    /// * `device_universe` - The absoulte DMX start universe of the device
    pub(crate) fn new(
        channel_numbers: ChannelParameter,
        default_value: ChannelValue,
        device_channel: ChannelIndex,
        device_universe: UniverseIndex,
    ) -> Self {

        let mut channel = channel_numbers.clone();
        channel.move_indices((0,0), (device_channel, device_universe));

        Channel {
            value: default_value,
            channel,
        }
    }

    //TODO Add the option to have some fixtures go over Universe-Borders
    /// Safely calculates a new absolute DMX address for a single channel when a fixture is moved.
    ///
    /// It determines the relative distance of the channel from the `old_start` address
    /// and reapplies this relative position to the `new_start` address.
    ///
    /// # Arguments
    ///
    /// * `channel`   - The current absolute channel and universe index to be moved.
    /// * `old_start` - The previous DMX start address and universe) of the fixture.
    /// * `new_start` - The new DMX start address and universe) of the fixture.
    fn move_single_channel(
        channel: ChannelInUniverse, old_start: ChannelInUniverse, new_start: ChannelInUniverse
    ) -> ChannelInUniverse {
        let max_c = MAX_CHANNEL as usize;

        let abs_channel = channel.1 * max_c + channel.0 as usize;
        let abs_old_start = old_start.1 * max_c + old_start.0 as usize;
        let abs_new_start = new_start.1 * max_c + new_start.0 as usize;

        let relative_pos = abs_channel.checked_sub(abs_old_start)
            .expect("CRITICAL: A channel was before the fixture's start channel!");
        let new_abs_channel = abs_new_start + relative_pos;

        let new_universe = new_abs_channel / max_c;
        let new_channel = (new_abs_channel % max_c) as ChannelIndex;

        (new_channel, new_universe)
    }

    /// Shifts all associated channel indices (coarse and fine) to a new DMX start address.
    ///
    /// # Arguments
    ///
    /// * `old_start` - The previous DMX start address and universe of the fixture.
    /// * `new_start` - The new DMX start address and universe to shift the channels to.
    pub(super) fn move_channels(&mut self, old_start: ChannelInUniverse, new_start: ChannelInUniverse) {
        self.channel.move_indices(old_start, new_start);
    }


    /// Returns the coarse DMX output value as `Vec<(ChannelInUniverse, 8-bit value)>` . If fine, ultra, uber, ... channels
    /// exist, then they are also part of the Return-Value
    pub(super) fn get_all_values(&self) -> Vec<(ChannelInUniverse, u8)> {
        let bytes = self.value.to_be_bytes();

        self.channel.get_channel_indices()
            .iter()
            .zip(bytes)
            .map(|(&channel, byte)| (channel, byte))
            .collect()
    }

    /// Returns a copy of the internal array containing all configured channel and universe indices.
    pub fn get_channel_indices(&self) -> ArrayVec<ChannelInUniverse, MAX_FINE_DEGREES> {
        self.channel.get_channel_indices()
    }

    /// Determines the default startup value for a given `SimplePropertyType`.
    ///
    /// To prevent fixtures from moving wildly on startup, spatial attributes like `Pan`
    /// and `Tilt` are initialized to their center positions (`ChannelValue::MAX / 2`).
    /// All other simple properties default to `0`.
    ///
    /// # Arguments
    ///
    /// * `property_type` - The [`SimplePropertyType`] for which to determine the default value.
    pub(super) fn get_default_value(property_type: SimplePropertyType) -> ChannelValue {
        match property_type {
            SimplePropertyType::Pan => ChannelValue::MAX / 2,
            SimplePropertyType::Tilt => ChannelValue::MAX / 2,
            _ => 0,
        }
    }
}


/// Represents a DMX-Channel parameter, managing the base channel and its fine-degree channels.
/// It uses an `ArrayVec` to store the coarse channel alongside optional fine channels up to [`MAX_FINE_DEGREES`].
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChannelParameter {
    channel: ArrayVec<ChannelInUniverse, MAX_FINE_DEGREES>
}

impl ChannelParameter {
    /// Initializes a new `ChannelParameter` with a single, coarse channel index.
    ///
    /// # Arguments
    ///
    /// * `channel_index` - The base (coarse) channel index to initialize the parameter with.
    /// * `universe_index` - The base (coarse) universe index to initialize the parameter with.
    pub fn new(channel_index: ChannelIndex, universe_index: UniverseIndex) -> Self {
        let mut channel = ArrayVec::new();
        channel.push((channel_index, universe_index));
        Self {
            channel,
        }
    }

    /// Adds a fine-degree channel to the parameter.
    ///
    /// The fine channels must be added in sequential order. The length of the internal
    /// channel array dictates which fine degree is expected next.
    ///
    /// # Arguments
    ///
    /// * `fine_degree` - The degree level of the fine channel (e.g., 1 for fine, 2 for ultra-fine).
    /// * `fine_index`  - The specific DMX-Channel and -Universe index for this fine degree.
    ///
    /// # Errors
    ///
    /// * [`FineDegreeOutOfRange`]((ChannelError::FineDegreeOutOfRange) - If the requested degree exceeds `MAX_FINE_DEGREES`.
    /// * [`FineDegreeExists`](ChannelError::FineDegreeExists) - If a channel for this fine degree has already been added.
    /// * [`FineDegreeTooHigh`](ChannelError::FineDegreeTooHigh) - If you attempt to add a higher fine degree before adding the intermediate ones.
    pub fn add_fine(
        &mut self, fine_degree: usize, fine_channel_index: ChannelIndex, fine_universe_index: UniverseIndex
    ) -> Result<(), ChannelError> {
        if fine_degree > MAX_FINE_DEGREES {
            return Err(ChannelError::FineDegreeOutOfRange(fine_degree));
        }

        let required_len = fine_degree;

        if self.channel.len() == required_len {
            self.channel.push((fine_channel_index, fine_universe_index));
            Ok(())
        } else if self.channel.len() > required_len {
            Err(ChannelError::FineDegreeExists(fine_degree))
        } else {
            Err(ChannelError::FineDegreeTooHigh(fine_degree))
        }
    }

    /// Shifts all managed channel indices to a new starting address.
    ///
    /// The method calculates the relative position of each channel based on the `old_start`
    /// and updates them relative to the `new_start`.
    ///
    /// # Arguments
    ///
    /// * `old_start` - The current DMX start address and universe used as the baseline for the shift.
    /// * `new_start` - The target DMX start address and universe to move the indices to.
    fn move_indices(&mut self, old_start: ChannelInUniverse, new_start: ChannelInUniverse) {
        let channels = &mut self.channel;

        for channel_index in channels.iter_mut() {
            *channel_index = Channel::move_single_channel(*channel_index, old_start, new_start);
        }
    }

    /// Returns a cloned `ArrayVec` containing all absolute channel and universe indices (coarse and fine)
    /// currently held by this parameter.
    pub(super) fn get_channel_indices(&self) -> ArrayVec<ChannelInUniverse, MAX_FINE_DEGREES> {
        self.channel.clone()
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

    /// Parses a raw string slice into a strongly typed `PropertyType`.
    ///
    /// It sequentially attempts to match the input string against known `ColorPropertyType`
    /// variants first, and then falls back to `SimplePropertyType` variants.
    ///
    /// # Arguments
    ///
    /// * `property_type` - A string slice representing the name of the property to parse.
    ///
    /// # Errors
    ///
    /// Returns a [`InvalidPropertyType-Error`](FixtureError::InvalidPropertyType) if the string matches neither
    /// a color nor a simple property.
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

impl SimplePropertyType {
    /// Attempts to parse a string identifier into a `SimplePropertyType`.
    ///
    /// Custom or manufacturer-specific properties can be parsed if they are
    /// prefixed with `other_` (e.g., `other_gobo_shake`).
    ///
    /// # Arguments
    ///
    /// * `s` - The raw string identifier to parse into a simple property.
    ///
    /// # Errors
    ///
    /// Returns a [`InvalidPropertyType-Error`](FixtureError::InvalidPropertyType) if the string does not match
    /// any known simple property and lacks the `other_` prefix.
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

/// Errors that can occur when reserving or accessing DMX-Channels.
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
