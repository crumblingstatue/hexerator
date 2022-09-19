use {
    egui_sfml::sfml::graphics::Color,
    serde::{Deserialize, Serialize},
};

#[derive(Serialize, Deserialize)]
pub struct MyColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl From<Color> for MyColor {
    fn from(Color { r, g, b, a }: Color) -> Self {
        Self { r, g, b, a }
    }
}

impl From<MyColor> for Color {
    fn from(MyColor { r, g, b, a }: MyColor) -> Self {
        Self { r, g, b, a }
    }
}
