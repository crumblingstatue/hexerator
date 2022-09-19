use {
    egui_sfml::sfml::graphics::Color,
    serde::{Deserialize, Serialize},
};

#[derive(Serialize, Deserialize)]
pub struct RgbaColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl From<Color> for RgbaColor {
    fn from(Color { r, g, b, a }: Color) -> Self {
        Self { r, g, b, a }
    }
}

impl From<RgbaColor> for Color {
    fn from(RgbaColor { r, g, b, a }: RgbaColor) -> Self {
        Self { r, g, b, a }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct RgbColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl RgbColor {
    pub const WHITE: Self = rgb(255, 255, 255);

    pub fn invert(&self) -> Self {
        rgb(!self.r, !self.g, !self.b)
    }
}

pub const fn rgb(r: u8, g: u8, b: u8) -> RgbColor {
    RgbColor { r, g, b }
}

impl From<RgbColor> for Color {
    fn from(src: RgbColor) -> Self {
        Self {
            r: src.r,
            g: src.g,
            b: src.b,
            a: 255,
        }
    }
}
