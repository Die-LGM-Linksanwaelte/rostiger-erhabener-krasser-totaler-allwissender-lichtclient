
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

    fn calculate_brightness(&self) -> u8 {
        (self.red+self.green+self.blue)/3
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