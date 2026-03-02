//#![allow(dead_code)]
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use crate::color::{Color, ColorType};
use crate::fixture::ChannelReservation::{Empty, Pending, Reserved};

pub struct FixtureList {
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

static DMX_CONFIGURATION: LazyLock<Mutex<Vec<[ChannelReservation<String, PropertyType>; 512]>>> = LazyLock::new(||{
    Mutex::new(Vec::new())
});

pub fn universe_count() -> usize {
    DMX_CONFIGURATION.lock().expect("Failed to lock DMX_CONFIGURATION").len()
}

pub fn ensure_universe_count(size: usize) {
    if size > universe_count() {
        let mut config = DMX_CONFIGURATION.lock().expect("Failed to lock DMX_CONFIGURATION");
        config.resize_with(size, || {
            std::array::from_fn(|_| Empty)
        })
    }
}

pub static FIXTURE_LIST: LazyLock<Mutex<FixtureList>> = LazyLock::new(|| {
    Mutex::new(FixtureList::new())
});

pub(crate) struct Channel{
    pub(crate) value: u16,
    channel : u16,
    fine_channel: Option<u16>,
}

impl Channel {
    pub(crate) fn new(
        channel_numbers: (u16, Option<u16>),
        default_value: u16,
        device_channel: u16
    ) -> Result<Self, ChannelError> {

        let channel = Self::checked_add(channel_numbers.0, device_channel)?;
        let fine_channel = if let Some(fine) = channel_numbers.1 {
            Some (
                Self::checked_add(fine, default_value)?
            )
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
            .filter(|&x| x <= 512)
            .ok_or(ChannelError::ChannelOutOfRange)
    }

    fn reserve_pending(&self, fixture_name: &str, universe: usize) -> Result<(), ChannelError> {
        let mut dmx_config = DMX_CONFIGURATION.lock().expect("Failed to lock DMX_CONFIGURATION");

        //Since ensure_universe_count should have been executed before, this Error should never occur, therefore it
        // should panic
        let universe = dmx_config.get_mut(universe - 1)
            .ok_or(ChannelError::UniverseOutOfRange).expect("Universe out of range");

        if let Reserved(existing,_) = universe[self.channel as usize].clone() {
            return Err(ChannelError::ChannelAlreadyInUse(existing));
        }

        if let Some(fine_channel) = self.fine_channel {
            if let Reserved(existing,_) = universe[fine_channel as usize].clone() {
                return Err(ChannelError::ChannelAlreadyInUse(existing));
            }

            universe[fine_channel as usize] = Pending(fixture_name.to_string());
        }
        universe[self.channel as usize] = Pending(fixture_name.to_string());

        Ok(())
    }

    fn reserve_final(&self, fixture_name: &str, universe: usize, property_type: PropertyType) {
        let mut dmx_config = DMX_CONFIGURATION.lock().expect("Failed to lock DMX_CONFIGURATION");

        //Since ensure_universe_count should have been executed before, this Error should never occur, therefore it
        // should panic
        let universe = dmx_config.get_mut(universe - 1)
            .ok_or(ChannelError::UniverseOutOfRange).expect("Universe out of range.");

        if let Pending(existing) = universe[self.channel as usize].clone() {
            if existing == fixture_name {
                universe[self.channel as usize] = Reserved(existing, property_type.clone());
            } else {
                panic!("A property of another fixture has been set to pending,\
                 cant reserve channel for {fixture_name}")
            }
        } else {
            panic!("Error: In {fixture_name}, a channel has not correctly been set to Pending. \
            This could happen if the fixture_type has multiple properties bound to the same channel.");
        }

        if let Some(fine_channel) = self.fine_channel {
            if let Pending(existing) = universe[fine_channel as usize].clone() {
                if existing == fixture_name {
                    universe[fine_channel as usize] = Reserved(existing, property_type);
                } else {
                    panic!("A property of another fixture has been set to pending,\
                 cant reserve fine-channel for {fixture_name}")
                }
            } else {
                panic!("Error: In {fixture_name}, a fine-channel has not correctly been set to Pending. \
            This could happen if the fixture_type has multiple properties bound to the same channel.");
            }
        }
    }

    fn get_default_value(property_type: PropertyType) -> u16 {
        match property_type {
            PropertyType::Pan => u16::MAX/2,
            PropertyType::Tilt => u16::MAX/2,
            _ => 0
        }
    }
}

/// Represents the various configurable properties of a lighting fixture.
///
/// This enum encapsulates all supported attribute types of a fixture,
/// allowing each property to carry its associated DMX channel(s).
/// It provides a unified way to describe colors, movement, beam effects,
/// gobos, atmospheric controls, and any custom or manufacturer-specific
/// attributes.
///
/// # Variants
///
/// * **Color(Color)**
///   A color-related property such as RGB, CMY, or other color-mixing systems.
///
/// * **Dimmer(Channel)**
///   Controls fixture brightness (0–255).
///
/// * **Strobe(Channel)**
///   Controls strobe rate or shutter pulse effects.
///
/// * **Beam { zoom, focus, frost }**
///   Beam-shaping properties:
///   - `zoom`: controls beam width
///   - `focus`: controls sharpness
///   - `frost`: applies diffusion/frost effect
///
/// * **Shutter(Channel)**
///   Mechanical shutter control (open/close).
///
/// * **Prism { prism, prism_rotation, prism_indexation }**
///   Prism and prism-effect controls:
///   - `prism`: enables/selects prism
///   - `prism_rotation`: rotation speed/direction
///   - `prism_indexation`: discrete index positioning
///
/// * **Gobo { gobo_rotation, gobo_rotation_speed, gobo_wheel_rotation, gobo_wheel_rotation_speed }**
///   Gobo selection and motion:
///   - `gobo_rotation`: absolute rotation
///   - `gobo_rotation_speed`: continuous rotation speed
///   - `gobo_wheel_rotation`: selects wheel slot rotation
///   - `gobo_wheel_rotation_speed`: rotation speed of the gobo wheel
///
/// * **Position { pan, tilt }**
///   Movement parameters for head-positioning.´
///
/// * **UV(Channel)**
///   UV-LED intensity control.
///
/// * **Speed(Channel)**
///   Global macro speed or effect speed.
///
/// * **Fog { fog_intensity, fog_fan_speed }**
///   Atmospheric effects:
///   - `fog_intensity`: fog output amount
///   - `fog_fan_speed`: fan speed for fog dispersion
///
/// * **Other(String, Channel)**
///   Any manufacturer-specific or unsupported property, given as a descriptive name and channel index.
#[derive(Hash,Eq,PartialEq,Clone)]
enum PropertyType {
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


pub struct FixtureType {
    color: Option<ColorType>,
    properties: HashMap<PropertyType, (u16, Option<u16>)>,
    name: String
}

pub struct Fixture {
    fixture_type: String,
    color: Option<Color>,
    properties: HashMap<PropertyType, Channel>,
    start_channel: u16,
    universe: usize,
    name: String,
}

impl FixtureType {
    pub fn new(name: String, properties: HashMap<String, (u16, Option<u16>)>) -> Result<Self, ParseError> {
        let mut color = ColorType::new();
        let mut new_properties = HashMap::new();

        for (key, value) in properties {
            if color.parse(key.clone(), value)? {
                continue
            }

            let property_type = PropertyType::from_string(&key)?;
            new_properties.insert(property_type, value);
        }

        let color = if color.exists() {
           Some(color)
        } else {
           None
        };

        Ok(FixtureType {
            color,
            properties: new_properties,
            name,
        })
    }
}

impl Fixture {
    pub fn new(fixture_type: &FixtureType, start_channel:u16, universe: usize, name:String) -> Result<Self, ChannelError> {
        ensure_universe_count(universe);
        let color = fixture_type.color.as_ref()
            .map(|c| {
                Color::new(c, start_channel)
            })
            .transpose()?;

        let properties = fixture_type.properties
            .iter()
            .map(|(property_type, channel)| {
                let default_value = Channel::get_default_value(property_type.clone());
                let channel = Channel::new(*channel, default_value, start_channel)?;
                channel.reserve_pending(&*name, universe)?;
                Ok((property_type.clone(), channel))
        }).collect::<Result<HashMap<PropertyType, Channel>,ChannelError>>()?;

        properties.iter().for_each(|(property_type, channel)| {
            channel.reserve_final(&*name, universe, property_type.clone());
        });

        Ok(Self {
            color,
            fixture_type: fixture_type.name.clone(),
            properties,
            start_channel,
            universe,
            name,
        })
    }

}

impl PropertyType {
    fn from_string(s: &str) -> Result<PropertyType, ParseError> {
        match s {
            "dimmer" => Ok(PropertyType::Dimmer),
            "strobe" => Ok(PropertyType::Strobe),
            "zoom" => Ok(PropertyType::Zoom),
            "focus" => Ok(PropertyType::Focus),
            "frost" => Ok(PropertyType::Frost),
            "prism" => Ok(PropertyType::Prism),
            "prism-rotation" => Ok(PropertyType::PrismRotation),
            "prism-index" => Ok(PropertyType::PrismIndexation),
            "gobo" => Ok(PropertyType::GoboRotation),
            "gobo-rotation" => Ok(PropertyType::GoboRotationSpeed),
            "gobo-wheel-rotation" => Ok(PropertyType::GoboWheelRotation),
            "gobo-wheel-speed" => Ok(PropertyType::GoboWheelRotationSpeed),
            "pan" => Ok(PropertyType::Pan),
            "tilt" => Ok(PropertyType::Tilt),
            "fog-intensity" => Ok(PropertyType::FogIntensity),
            "fog-fan-speed" => Ok(PropertyType::FogFanSpeed),
            "shutter" => Ok(PropertyType::Shutter),
            "uv" => Ok(PropertyType::UV),
            "speed" => Ok(PropertyType::Speed),
            _ => {
                if let Some(suffix) = s.strip_prefix("other_") {
                    Ok(PropertyType::Other(suffix.to_string()))
                } else {
                    Err(ParseError::InvalidPropertyType(s.to_string()))
                }
            }
        }
    }
}

#[derive(Clone)]
enum ChannelReservation<T, U> {
    Empty,
    Pending(T),
    Reserved(T, U),
}

#[derive(Debug)]
pub enum ParseError {
    InvalidPropertyType(String),
    MultipleColorOutputTypes(String),
}

#[derive(Debug)]
pub enum ChannelError {
    ChannelOutOfRange,
    UniverseOutOfRange,
    ChannelAlreadyInUse(String),
}