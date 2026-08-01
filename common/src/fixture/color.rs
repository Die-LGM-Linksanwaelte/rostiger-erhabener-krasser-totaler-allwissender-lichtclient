use std::cmp::{max, min, PartialEq};
use std::fmt::{Display, Formatter};
use serde::{Deserialize, Serialize};
use OutputType::{CMY, HSV, RGB};
use crate::fixture::{Channel, ChannelError, ChannelIndex, ChannelValue, FixtureError, FloatChannelValue, PropertyType, SignedChannelValue};
use crate::fixture::channel::ChannelParameter;



/// Represents a color with its channel values for all supported color models
/// (RGB, CMY, and HSV).
pub struct Color {
    output_type: OutputType,
    color1: Option<Channel>,
    color2: Option<Channel>,
    color3: Option<Channel>,
    red: ChannelValue,
    green: ChannelValue,
    blue: ChannelValue,
    cyan: ChannelValue,
    magenta: ChannelValue,
    yellow: ChannelValue,
    hue: ChannelValue,
    saturation: ChannelValue,
    value: ChannelValue,
}

/// A color template used by [`FixtureType`] to define how colors are
/// generated for fixtures of that type.
///
/// Each color channel holds a DMX value and an optional second value
/// for fine-grained 16-bit control
#[derive(Debug, Serialize, Deserialize)]
pub struct ColorType {
    output_type: Option<OutputType>,
    color1: Option<ChannelParameter>,
    color2: Option<ChannelParameter>,
    color3: Option<ChannelParameter>,
}
#[derive(Debug, PartialEq, Copy, Clone, Serialize, Deserialize)]
enum OutputType {
    RGB,
    HSV,
    CMY,
}

/// Identifies a specific channel or property of a [`Color`].
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum ColorPropertyType {
    Red,
    Green,
    Blue,
    Cyan,
    Magenta,
    Yellow,
    Hue,
    Saturation,
    Value,
}

impl ColorPropertyType {
    fn new(color_number: u8, output_type: OutputType) -> Option<ColorPropertyType> {
        match (color_number, output_type) {
            (1, RGB) => Some(ColorPropertyType::Red),
            (2, RGB) => Some(ColorPropertyType::Green),
            (3, RGB) => Some(ColorPropertyType::Blue),

            (1, CMY) => Some(ColorPropertyType::Cyan),
            (2, CMY) => Some(ColorPropertyType::Magenta),
            (3, CMY) => Some(ColorPropertyType::Yellow),

            (1, HSV) => Some(ColorPropertyType::Hue),
            (2, HSV) => Some(ColorPropertyType::Saturation),
            (3, HSV) => Some(ColorPropertyType::Value),

            _ => None,
        }
    }

    fn to_output_type(&self) -> (u8, OutputType) {
        match self {
            ColorPropertyType::Red => (1, RGB),
            ColorPropertyType::Green => (2, RGB),
            ColorPropertyType::Blue => (3, RGB),

            ColorPropertyType::Cyan => (1, CMY),
            ColorPropertyType::Magenta => (2, CMY),
            ColorPropertyType::Yellow => (3, CMY),

            ColorPropertyType::Hue => (1, HSV),
            ColorPropertyType::Saturation => (2, HSV),
            ColorPropertyType::Value => (3, HSV),
        }
    }

    /// Parses a [`ColorPropertyType`] from a string.
    ///
    /// # Arguments
    ///
    /// * `property` - The property name (e.g. `"red"`, `"hue"`, `"saturation"`)
    ///
    /// # Errors
    ///
    /// Returns [`FixtureError::InvalidPropertyType`] if `property` does not
    /// match any known color property.
    pub fn from_string(property: &str) -> Result<ColorPropertyType, FixtureError> {
        match property {
            "red" => Ok(ColorPropertyType::Red),
            "green" => Ok(ColorPropertyType::Green),
            "blue" => Ok(ColorPropertyType::Blue),
            "cyan" => Ok(ColorPropertyType::Cyan),
            "magenta" => Ok(ColorPropertyType::Magenta),
            "yellow" => Ok(ColorPropertyType::Yellow),
            "hue" => Ok(ColorPropertyType::Hue),
            "saturation" => Ok(ColorPropertyType::Saturation),
            "value" => Ok(ColorPropertyType::Value),
            _ => Err(FixtureError::InvalidPropertyType(property.to_string())),
        }
    }
}

impl Display for ColorPropertyType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ColorPropertyType::Red => write!(f, "red"),
            ColorPropertyType::Green => write!(f, "green"),
            ColorPropertyType::Blue => write!(f, "blue"),
            ColorPropertyType::Cyan => write!(f, "cyan"),
            ColorPropertyType::Magenta => write!(f, "magenta"),
            ColorPropertyType::Yellow => write!(f, "yellow"),
            ColorPropertyType::Hue => write!(f, "hue"),
            ColorPropertyType::Saturation => write!(f, "saturation"),
            ColorPropertyType::Value => write!(f, "value"),
        }
    }
}

impl ColorType {
    /// Creates an empty [`ColorType`] with no output type or channels set.
    pub fn new() -> Self {
        Self {
            output_type: None,
            color1: None,
            color2: None,
            color3: None,
        }
    }

    /// Parses a color channel name and assigns its DMX value to the corresponding slot.
    ///
    /// Accepts `"red"`, `"green"`, `"blue"` (RGB), `"cyan"`, `"magenta"`, `"yellow"` (CMY),
    /// or `"hue"`, `"saturation"`, `"value"` (HSV).
    ///
    /// Returns `Ok(true)` if the channel was recognized and set, `Ok(false)` if the
    /// channel name is unknown.
    ///
    /// # Errors
    ///
    /// Returns [`FixtureError::MultipleColorOutputTypes`] if the channel belongs to a
    /// different color model than one already assigned (e.g. mixing RGB and HSV).
    pub fn checked_add_channel(&mut self, s: ColorPropertyType, value: ChannelParameter) -> Result<(), FixtureError> {
        let (new_type, slot) = match s {
            ColorPropertyType::Red          => (RGB, 1),
            ColorPropertyType::Green        => (RGB, 2),
            ColorPropertyType::Blue         => (RGB, 3),

            ColorPropertyType::Cyan         => (CMY, 1),
            ColorPropertyType::Magenta      => (CMY, 2),
            ColorPropertyType::Yellow       => (CMY, 3),

            ColorPropertyType::Hue          => (HSV, 1),
            ColorPropertyType::Saturation   => (HSV, 2),
            ColorPropertyType::Value        => (HSV, 3),
        };

        if let Some(old_type) = self.output_type {
            if old_type != new_type {
                return Err(FixtureError::MultipleColorOutputTypes(format!(
                    "{s} is incompatible with {:?}",
                    old_type
                )));
            }
        }

        self.output_type = Some(new_type);

        let target = match slot {
            1 => &mut self.color1,
            2 => &mut self.color2,
            3 => &mut self.color3,
            _ => unreachable!(),
        };

        *target = Some(value);

        Ok(())
    }

    /// Returns `true` if at least one color channel has been set.
    pub fn exists(&self) -> bool {
        self.output_type.is_some()
    }
}

impl Color {
    /// Creates a [`Color`] from a [`ColorType`] template and reserves the required DMXChannels.
    ///
    /// Called internally by [`Fixture::new`].
    ///
    /// The default channel value depends on the color model: CMY channels default to
    /// `u16::MAX` (full), all others default to `0`.
    ///
    /// # Arguments
    ///
    /// * `color_type`      - The template defining which channels and color model to use
    /// * `device_channel`  - The DMXChannel offset of the device
    /// * `universe`        - The DMX universe to reserve channels in
    /// * `fixture_name`    - The fixture name, used for channel reservation
    ///
    /// # Errors
    ///
    /// Returns a [`ChannelError`] if any channel could not be created or reserved.
    pub fn new(
        color_type: &ColorType,
        device_channel: u16,
        universe: usize,
        fixture_name: &str,
    ) -> Result<Self, ChannelError> {
        let default_value = if color_type.output_type == Some(CMY) {
            ChannelValue::MAX
        } else if let Some(_) = color_type.output_type {
            0 as ChannelValue
        } else {
            // ColorType::new() must only be called when ColorType::exists() returns true
            unreachable!();
        };

        let color1 = color_type.color1.clone()
            .map(|c| Channel::new(c, default_value, device_channel))
            .transpose()?;
        let color2 = color_type.color2.clone()
            .map(|c| Channel::new(c, default_value, device_channel))
            .transpose()?;
        let color3 = color_type.color3.clone()
            .map(|c| Channel::new(c,default_value, device_channel))
            .transpose()?;

        let output_type = color_type.output_type.unwrap();

        if let Some(color) = &color1 {
            color.reserve_pending(fixture_name, universe)?;
        }
        if let Some(color) = &color2 {
            color.reserve_pending(fixture_name, universe)?;
        }
        if let Some(color) = &color3 {
            color.reserve_pending(fixture_name, universe)?;
        }

        if let Some(color) = &color1 {
            let property = PropertyType::Color(ColorPropertyType::new(1, output_type).unwrap());
            color.reserve_final(fixture_name, universe, property);
        }
        if let Some(color) = &color2 {
            let property = PropertyType::Color(ColorPropertyType::new(2, output_type).unwrap());
            color.reserve_final(fixture_name, universe, property);
        }
        if let Some(color) = &color3 {
            let property = PropertyType::Color(ColorPropertyType::new(3, output_type).unwrap());
            color.reserve_final(fixture_name, universe, property);
        }

        Ok(Self {
            output_type,
            color1,
            color2,
            color3,
            red: 0 as ChannelValue,
            green: 0 as ChannelValue,
            blue: 0 as ChannelValue,
            cyan: ChannelValue::MAX,
            magenta: ChannelValue::MAX,
            yellow: ChannelValue::MAX,
            hue: 0 as ChannelValue,
            saturation: 0 as ChannelValue,
            value: 0 as ChannelValue,
        })
    }
    
    fn set_color(&mut self) {
        let (v1, v2, v3) = match self.output_type {
            RGB => (self.red, self.green, self.blue),
            HSV => (self.hue, self.saturation, self.value),
            CMY => (self.cyan, self.magenta, self.red),
        };
        if let Some(c) = self.color1.as_mut() {
            c.value = v1
        }
        if let Some(c) = self.color2.as_mut() {
            c.value = v2
        }
        if let Some(c) = self.color3.as_mut() {
            c.value = v3
        }
    }

    fn set_rgb(&mut self, red: ChannelValue, green: ChannelValue, blue: ChannelValue) {
        self.red = red;
        self.green = green;
        self.blue = blue;

        self.cyan = ChannelValue::MAX - red;
        self.magenta = ChannelValue::MAX - green;
        self.yellow = ChannelValue::MAX - blue;

        let max = max(red, max(green, blue));
        let min = min(red, min(green, blue));
        let delta = max - min;
        self.value = max;
        self.saturation = if max == 0 {
            0
        } else {
            ((delta as FloatChannelValue * ChannelValue::MAX as FloatChannelValue) / max as FloatChannelValue).round()
                as ChannelValue
        };
        let mut hue: SignedChannelValue = (ChannelValue::MAX as FloatChannelValue / 6.0 as FloatChannelValue
            * (if delta == 0 {
                0.0
            } else if max == red {
                ((green as FloatChannelValue - blue as FloatChannelValue) / delta as FloatChannelValue) % 6.0
            } else if max == green {
                ((blue as FloatChannelValue - red as FloatChannelValue) / delta as FloatChannelValue) + 2.0
            } else {
                ((red as FloatChannelValue - green as FloatChannelValue) / delta as FloatChannelValue) + 4.0
            })) as SignedChannelValue;

        if hue < 0 {
            hue = hue + ChannelValue::MAX as SignedChannelValue;
        }

        self.hue = hue as ChannelValue;

        self.set_color()
    }

    fn set_hsv(&mut self, hue: ChannelValue, saturation: ChannelValue, value: ChannelValue) {
        self.hue = hue;
        self.saturation = saturation;
        self.value = value;
        //Achtung: es folgt eine unstetige Kackfunktion
        let c = (value as FloatChannelValue *
            (saturation as FloatChannelValue / ChannelValue::MAX as FloatChannelValue)).round() as ChannelValue;
        let m = value.saturating_sub(c);
        let h = hue as FloatChannelValue / (ChannelValue::MAX as FloatChannelValue / 6 as FloatChannelValue);
        let x = (c as FloatChannelValue * (1.0 - ((h % 2.0) - 1.0).abs()) ).round() as ChannelValue;

        let (r, g, b) = match h {
            n if n < 1.0 => (c, x, 0),
            n if n < 2.0 => (x, c, 0),
            n if n < 3.0 => (0, c, x),
            n if n < 4.0 => (0, x, c),
            n if n < 5.0 => (x, 0, c),
            n if n <= 6.0 => (c, 0, x),
            _ => (0, 0, 0),
        };

        self.red = r.saturating_add(m);
        self.green = g.saturating_add(m);
        self.blue = b.saturating_add(m);

        self.cyan = ChannelValue::MAX - self.red;
        self.magenta = ChannelValue::MAX - self.green;
        self.yellow= ChannelValue::MAX - self.blue;
        
        self.set_color()
    }

    /// Sets a single color property and updates all DMXChannels accordingly.
    ///
    /// CMY values are converted to RGB internally (`u16::MAX - value`),
    /// HSV values are converted to RGB via [`Color::set_hsv`].
    /// RGB values are converted to HSV via ['Color::set_rgb']
    pub fn set(&mut self, property: ColorPropertyType, value: ChannelValue) {
        let (color_number, output_type) = property.to_output_type();

        let (mut value1, mut value2, mut value3) = match output_type {
            RGB => (self.red, self.green, self.blue),
            CMY => (self.cyan, self.magenta, self.yellow),
            HSV => (self.hue, self.saturation, self.value),
        };

        match color_number {
            1 => value1 = value,
            2 => value2 = value,
            3 => value3 = value,
            _ => unreachable!(),
        }

        match output_type {
            RGB => self.set_rgb(value1, value2, value3),
            HSV => self.set_hsv(value1, value2, value3),
            CMY => self.set_rgb(
                ChannelValue::MAX - value1,
                ChannelValue::MAX - value2,
                ChannelValue::MAX - value3
            ),
        }
    }

    /// Returns the current DMX output values of all active color channels.
    ///
    /// Each entry is a `(value, channel_index)` pair. For channels with 16-bit
    /// fine control, two entries are returned — coarse first, then fine.
    pub fn get_values(&self) -> Vec<(ChannelIndex, u8)> {
        let mut output = Vec::new();

        if let Some(c) = self.color1.as_ref() {
            output.extend(c.get_all_values());
        }

        if let Some(c) = self.color2.as_ref() {
            output.extend(c.get_all_values());
        }

        if let Some(c) = self.color3.as_ref() {
            output.extend(c.get_all_values());
        }

        output
    }
}
