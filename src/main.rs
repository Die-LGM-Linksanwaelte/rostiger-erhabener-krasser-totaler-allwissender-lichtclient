
use std::cmp::{max, min};

fn main() {
    println!("Hello, world!");
}


struct Color {
    red: u8,
    green: u8,
    blue: u8
}

impl Color {

    fn new(red:u8,green:u8,blue:u8) -> Color {
        Color {red,green,blue}
    }

    fn make_brighter(&mut self, scale:f32) {
        self.red = min(max(((self.red as f32) * scale) as u8, 0),255);
        self.green = min(max(((self.green as f32) * scale) as u8, 0),255);
        self.blue = min(max(((self.blue as f32) * scale) as u8, 0),255);
    }

    fn calculate_hsv(&self) -> (u16, u8, u8) {
        let max = max(self.red, max(self.green, self.blue));
        let min = min(self.red, min(self.green, self.blue));
        let delta = max - min;
        let saturation = if max != 0 {
            (255.0 * (delta as f32 / max as f32)) as u8
        } else {
            0
        };
        let value = max;
        let mut hue: i32 = (60 * if delta == 0 {
            0.0
        } else if max == self.red {
            ((self.green - self.blue) as f32 / delta as f32) % 6.0
        } else if max == self.green {
            ((self.blue - self.red) as f32 / delta as f32 ) + 2.0
        } else {
            ((self.red - self.green) as f32 / delta as f32) + 4.0
        }) as i32;

        if hue < 0 {
            hue = hue + 360;
        }
        let hue = hue as u16;

        (hue, saturation, value)
    }

    fn calculate_saturation(&self) -> u8 {
        let delta = self.calculate_brightness() as f32
            - min(self.red, min(self.green, self.blue)) as f32;
        ((delta / self.calculate_brightness() as f32) * 255.0) as u8
    }

}

enum PropertyKind {
    Dimmer(u8),
    RGB(Color),
    CMY(Color),
    HSV(Color),
    Strobe(u8)
}

struct Property {
    kind: PropertyKind,
    dmx_offset: u8,
}

impl Property {
    fn set_color(&mut self, color:&Color) {
        match &mut self.kind {
            PropertyKind::RGB(c)
            | PropertyKind::CMY(c)
            | PropertyKind::HSV(c) => {
                c.red = color.red;
                c.green = color.green;
                c.blue = color.blue;
            }
            _ => {}
        }
    }
}

struct Fixture {
    properties: Vec<Property>
}

impl Fixture {

    fn get_length(&self) -> usize {
        self.properties.iter().max_by_key(|p| p.dmx_offset).unwrap().dmx_offset as usize
    }

    fn set_color(&mut self, color: Color) {
        for prop in &mut self.properties {
            prop.set_color(&color);
        }
    }

    fn set_dimer(&mut self, value: u8) {
        let found_dimmer = false;
        for prop in &mut self.properties {
            match &mut prop.kind {
                PropertyKind::Dimmer(dimer) {
                    found_dimmer = true;
                    dimmer = value; //TODO
                }
                _ => {}
            }
        }

        if !found_dimmer {
            //TODO
        }
    }

}

enum Light {
    Dimmer(u8),
    DimmerAndColor(u8, Color),
    Color(Color)


}

impl Light {

    fn set_color(self, red: u8, green: u8, blue: u8) {
        match self {
            Light::Color(mut color) => { color = Color::new(red,green,blue) }
            Light::DimmerAndColor(dimmer, mut color) => { color = Color::new(red,green,blue) }
            Light::Dimmer(dimmer) => {}
        }
    }

    fn set_brightness(self, brightness: u8) {
        match self {
            Light::Color(mut color) => {
                let old_brightness= color.calculate_brightness() as f32;
                color.make_brighter((brightness as f32)/old_brightness);
            }
            Light::DimmerAndColor(mut dimmer, mut _color) => {
                dimmer = brightness
            }
            Light::Dimmer(mut dimmer) => {
                dimmer = brightness
            }
        }
    }
}