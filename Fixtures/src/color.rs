use std::cmp::{max, min};

pub struct Color {
    red: crate::fixture::Channel,
    green: crate::fixture::Channel,
    blue: crate::fixture::Channel,
    cyan: crate::fixture::Channel,
    magenta: crate::fixture::Channel,
    yellow: crate::fixture::Channel,
    hue: crate::fixture::Channel,
    saturation: crate::fixture::Channel,
    value: crate::fixture::Channel,
}

impl Color {
    fn set_rgb(&mut self, red: u16, green: u16, blue: u16) {
        self.red.value = red;
        self.green.value = green;
        self.blue.value = blue;

        self.cyan.value = red - u16::MAX;
        self.magenta.value = green - u16::MAX;
        self.yellow.value = blue - u16::MAX;

        let max = max(red, max(green, blue));
        let min = min(red, min(green, blue));
        let delta = max - min;
        self.value.value = max;
        self.saturation.value = if max == 0 {
            0
        } else {
            (delta * u16::MAX) / max
        };
        let mut hue: i32 = (u16::MAX as f32 / 6.0
            * (if delta == 0 {
                0.0
            } else if max == red {
                ((green - blue) as f32 / delta as f32) % 6.0
            } else if max == green {
                ((blue - red) as f32 / delta as f32) + 2.0
            } else {
                ((red - green) as f32 / delta as f32) + 4.0
            })) as i32;

        if hue < 0 {
            hue = hue + u16::MAX as i32;
        }

        self.hue.value = hue as u16;
    }

    fn set_hsv(&mut self, hue: u16, saturation: u16, value: u16) {
        self.hue.value = hue;
        self.saturation.value = saturation;
        self.value.value = value;
        //Achtung: es folgt eine unstetige Kackfunktion
        let c = value * saturation;
        let h = hue as f32 / (u16::MAX as f32 / 6f32);
        let x = c as f32 * (1.0 - ((h % 2.0) - 1.0).abs());
        match h {
            n if n < 1.0 => {
                self.red.value = c;
                self.green.value = x as u16;
                self.blue.value = 0;
            }
            n if n >= 1.0 && n < 2.0 => {
                self.red.value = x as u16;
                self.green.value = c;
                self.blue.value = 0;
            }
            n if n >= 2.0 && n < 3.0 => {
                self.red.value = 0;
                self.green.value = c;
                self.blue.value = x as u16;
            }
            n if n >= 3.0 && n < 4.0 => {
                self.red.value = 0;
                self.green.value = x as u16;
                self.blue.value = c;
            }
            n if n >= 4.0 && n < 5.0 => {
                self.red.value = x as u16;
                self.green.value = 0;
                self.blue.value = c;
            }
            n if n >= 5.0 && n < 6.0 => {
                self.red.value = c;
                self.green.value = 0;
                self.blue.value = x as u16;
            }
            _ => {}
        }
        self.cyan.value = self.red.value - u16::MAX;
        self.magenta.value = self.green.value - u16::MAX;
        self.yellow.value = self.blue.value - u16::MAX;
    }
}
