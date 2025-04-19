use {
    crate::{
        color::{RgbColor, RgbaColor},
        view::{ViewportScalar, ViewportVec},
    },
    egui_sf2g::sf2g::graphics::Color,
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

impl TryFrom<egui_sf2g::sf2g::system::Vector2<i32>> for ViewportVec {
    type Error = <ViewportScalar as TryFrom<i32>>::Error;

    fn try_from(sf_vec: egui_sf2g::sf2g::system::Vector2<i32>) -> Result<Self, Self::Error> {
        Ok(Self {
            x: sf_vec.x.try_into()?,
            y: sf_vec.y.try_into()?,
        })
    }
}
