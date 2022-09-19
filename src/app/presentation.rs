use {
    crate::{
        color::{rgba, RgbaColor},
        value_color::ColorMethod,
    },
    serde::{Deserialize, Serialize},
};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Presentation {
    pub color_method: ColorMethod,
    pub invert_color: bool,
    pub sel_color: RgbaColor,
    pub cursor_color: RgbaColor,
    pub cursor_active_color: RgbaColor,
}

impl Default for Presentation {
    fn default() -> Self {
        Self {
            color_method: ColorMethod::Default,
            invert_color: false,
            sel_color: rgba(75, 75, 75, 255),
            cursor_color: rgba(160, 160, 160, 255),
            cursor_active_color: rgba(255, 255, 255, 255),
        }
    }
}
