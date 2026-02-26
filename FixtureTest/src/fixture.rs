use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use crate::color::Color;

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

pub static FIXTURE_LIST: LazyLock<Mutex<FixtureList>> = LazyLock::new(|| {
    Mutex::new(FixtureList::new())
});

pub(crate) struct Channel{
    pub(crate) value: u16,
    channel : u16,
    fine_channel: Option<u16>,
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
#[derive(Hash,Eq,PartialEq)]
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
    properties: HashMap<PropertyType, (u16, Option<u16>)>,
    name: String
}

pub struct Fixture {
    fixture_type: FixtureType,
    properties: HashMap<PropertyType, Channel>,
    start_channel: u16,
    name: String,
}

impl FixtureType {
    pub fn new(name: String, properties: HashMap<String, (u16, Option<u16>)>) -> Result<Self, ParseError> {
        let properties = properties
            .into_iter()
            .map(|(key, value)| {
            let property_type = PropertyType::from_string(&key)?;
            Ok((property_type, value))
        }).collect::<Result<HashMap<_, _>, ParseError>>()?;
        Ok(FixtureType {
            properties,
            name,
        })
    }
}

impl Fixture {
    fn new(fixture_type: FixtureType, start_channel:u16, name:String) -> Self {
        Fixture {
            fixture_type,
            properties: HashMap::new(),
            start_channel,
            name,
        }
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

#[derive(Debug)]
pub enum ParseError {
    InvalidPropertyType(String),
}

