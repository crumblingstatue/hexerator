use {
    crate::{color::MyColor, value_color::ColorMethod},
    egui_sfml::sfml::graphics::Color,
    serde::{Deserialize, Serialize},
    serde_with::{serde_as, FromInto},
};

#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Presentation {
    pub color_method: ColorMethod,
    pub invert_color: bool,
    #[serde_as(as = "FromInto<MyColor>")]
    pub sel_color: Color,
    #[serde_as(as = "FromInto<MyColor>")]
    pub cursor_color: Color,
    #[serde_as(as = "FromInto<MyColor>")]
    pub cursor_active_color: Color,
}

impl Default for Presentation {
    fn default() -> Self {
        Self {
            color_method: ColorMethod::Default,
            invert_color: false,
            sel_color: Color::rgb(75, 75, 75),
            cursor_color: Color::rgb(160, 160, 160),
            cursor_active_color: Color::WHITE,
        }
    }
}
