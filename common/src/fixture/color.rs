use std::cmp::{max, min, PartialEq};
use serde::{Deserialize, Serialize};
use OutputType::{CMY, HSV, RGB};
use crate::fixture::{Channel, ChannelError, FixtureError, PropertyType};

/// Represents a color with its channel values for all supported color models
/// (RGB, CMY, and HSV).
pub struct Color {
    output_type: OutputType,
    color1: Option<Channel>,
    color2: Option<Channel>,
    color3: Option<Channel>,
    red: u16,
    green: u16,
    blue: u16,
    cyan: u16,
    magenta: u16,
    yellow: u16,
    hue: u16,
    saturation: u16,
    value: u16,
}

/// A color template used by [`FixtureType`] to define how colors are
/// generated for fixtures of that type.
///
/// Each color channel holds a DMX value and an optional second value
/// for fine-grained 16-bit control
#[derive(Debug, Serialize, Deserialize)]
pub struct ColorType {
    output_type: Option<OutputType>,
    color1: Option<(u16, Option<u16>)>,
    color2: Option<(u16, Option<u16>)>,
    color3: Option<(u16, Option<u16>)>,
}
#[derive(Debug, PartialEq, Copy, Clone, Serialize, Deserialize)]
enum OutputType {
    RGB,
    HSV,
    CMY
}

/// Identifies a specific channel or property of a [`Color`].
#[derive(Clone, Debug)]
pub enum ColorPropertyType {
    Red,
    Green,
    Blue,
    Cyan,
    Magenta,
    Yellow,
    Hue,
    Saturation,
    Value
}

impl ColorPropertyType {
    fn new(color_number: u16, output_type: OutputType) -> Option<ColorPropertyType> {
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

            _ => None
        }
    }

    fn to_output_type(&self) -> (u16, OutputType) {
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
            _ => Err(FixtureError::InvalidPropertyType(property.to_string()))
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
    pub fn parse(&mut self, s: String, value: (u16, Option<u16>)) -> Result<bool, FixtureError> {
        let (new_type, slot) = match s.as_str() {
            "red"        => (RGB, 1),
            "green"      => (RGB, 2),
            "blue"       => (RGB, 3),

            "cyan"       => (CMY, 1),
            "magenta"    => (CMY, 2),
            "yellow"     => (CMY, 3),

            "hue"        => (HSV, 1),
            "saturation" => (HSV, 2),
            "value"      => (HSV, 3),

            _ => return Ok(false),
        };

        if let Some(old_type) = self.output_type {
            if old_type != new_type {
                return Err(FixtureError::MultipleColorOutputTypes(
                   format!("{s} is incompatible with {:?}", old_type)
                ));
            }
        }

        self.output_type = Some(new_type);

        let target = match slot {
            1 => &mut self.color1,
            2 => &mut self.color2,
            3 => &mut self.color3,
            _ => unreachable!()
        };

        *target = Some(value);

        Ok(true)
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
        color_type: &ColorType, device_channel: u16, universe: usize, fixture_name: &str
    ) -> Result<Self, ChannelError> {
        let default_value = if color_type.output_type == Some(CMY) {
            u16::MAX
        } else if let Some(_) = color_type.output_type {
            0
        } else {
            // ColorType::new() must only be called when ColorType::exists() returns true
            unreachable!();
        };

        let color1 = color_type.color1
            .map(|c| Channel::new(c, default_value, device_channel))
            .transpose()?;
        let color2 = color_type.color2
            .map(|c| Channel::new(c, default_value, device_channel))
            .transpose()?;
        let color3 = color_type.color3
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
            red: 0,
            green: 0,
            blue: 0,
            cyan: u16::MAX,
            magenta: u16::MAX,
            yellow: u16::MAX,
            hue: 0,
            saturation: 0,
            value: 0,

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

    fn set_rgb(&mut self, red: u16, green: u16, blue: u16) {
        self.red = red;
        self.green = green;
        self.blue = blue;

        self.cyan = u16::MAX - red;
        self.magenta = u16::MAX - green;
        self.yellow = u16::MAX - blue;

        let max = max(red, max(green, blue));
        let min = min(red, min(green, blue));
        let delta = max - min;
        self.value = max;
        self.saturation = if max == 0 {
            0
        } else {
            ( (delta as f32 * u16::MAX as f32) / max as f32 ).round() as u16
        };
        let mut hue: i32 = (u16::MAX as f32 / 6.0_f32
            * (if delta == 0 {
                0.0
            } else if max == red {
                ((green as f32 - blue as f32) / delta as f32) % 6.0
            } else if max == green {
                ((blue as f32 - red as f32) / delta as f32) + 2.0
            } else {
                ((red as f32 - green as f32) / delta as f32) + 4.0
            })) as i32;

        if hue < 0 {
            hue = hue + u16::MAX as i32;
        }

        self.hue = hue as u16;
        
        self.set_color()
    }

    fn set_hsv(&mut self, hue: u16, saturation: u16, value: u16) {
        self.hue = hue;
        self.saturation = saturation;
        self.value = value;
        //Achtung: es folgt eine unstetige Kackfunktion
        let c = (value as f32 * (saturation as f32 / u16::MAX as f32)).round() as u16;
        let m = value.saturating_sub(c);
        let h = hue as f32 / (u16::MAX as f32 / 6f32);
        let x = ( c as f32 * (1.0 - ((h % 2.0) - 1.0).abs()) ).round() as u16;

        let (r, g, b) = match h {
            n if n < 1.0 => (c, x, 0),
            n if n < 2.0 => (x, c, 0),
            n if n < 3.0 => (0, c, x),
            n if n < 4.0 => (0, x, c),
            n if n < 5.0 => (x, 0, c),
            n if n <= 6.0 => (c, 0, x),
            _ => (0, 0, 0)
        };

        self.red = r.saturating_add(m);
        self.green = g.saturating_add(m);
        self.blue = b.saturating_add(m);

        self.cyan = u16::MAX - self.red;
        self.magenta = u16::MAX - self.green;
        self.yellow= u16::MAX - self.blue;
        
        self.set_color()
    }

    /// Sets a single color property and updates all DMXChannels accordingly.
    ///
    /// CMY values are converted to RGB internally (`u16::MAX - value`),
    /// HSV values are converted to RGB via [`Color::set_hsv`].
    /// RGB values are converted to HSV via ['Color::set_rgb']
    pub fn set(&mut self, property: ColorPropertyType, value: u16) {
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
                u16::MAX - value1,
                u16::MAX - value2,
                u16::MAX - value3
            ),
        }



    }

    /// Returns the current DMX output values of all active color channels.
    ///
    /// Each entry is a `(value, channel_index)` pair. For channels with 16-bit
    /// fine control, two entries are returned — coarse first, then fine.
    pub fn get_values(&self) -> Vec<(u16, u8)> {
        let mut output = Vec::new();

        if let Some(c) = self.color1.as_ref() {
            output.push(c.get_value());
            if let Some(fine_value) = c.get_fine_value() {
                output.push(fine_value);
            }
        }

        if let Some(c) = self.color2.as_ref() {
            output.push(c.get_value());
            if let Some(fine_value) = c.get_fine_value() {
                output.push(fine_value);
            }
        }

        if let Some(c) = self.color3.as_ref() {
            output.push(c.get_value());
            if let Some(fine_value) = c.get_fine_value() {
                output.push(fine_value);
            }
        }


        output

    }
}
