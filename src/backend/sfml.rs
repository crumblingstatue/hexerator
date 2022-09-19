use {
    crate::color::{RgbColor, RgbaColor},
    egui_sfml::sfml::graphics::Color,
};

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
