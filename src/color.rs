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
