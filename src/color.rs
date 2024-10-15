use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct RgbaColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}
impl RgbaColor {
    pub(crate) fn with_as_egui_mut(&mut self, f: impl FnOnce(&mut egui::Color32)) {
        let mut ec = self.to_egui();
        f(&mut ec);
        *self = Self::from_egui(ec);
    }
    fn from_egui(c: egui::Color32) -> Self {
        Self {
            r: c.r(),
            g: c.g(),
            b: c.b(),
            a: c.a(),
        }
    }
    fn to_egui(self) -> egui::Color32 {
        egui::Color32::from_rgba_premultiplied(self.r, self.g, self.b, self.a)
    }
}

pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> RgbaColor {
    RgbaColor { r, g, b, a }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
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
