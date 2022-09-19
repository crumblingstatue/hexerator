use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct RgbaColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> RgbaColor {
    RgbaColor { r, g, b, a }
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
