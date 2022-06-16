use sfml::graphics::Color;

use crate::color::ColorMethod;

#[derive(Debug)]
pub struct Presentation {
    pub color_method: ColorMethod,
    pub invert_color: bool,
    pub bg_color: [f32; 3],
    pub sel_color: Color,
    pub cursor_color: Color,
    pub cursor_active_color: Color,
}

impl Default for Presentation {
    fn default() -> Self {
        Self {
            color_method: ColorMethod::Default,
            invert_color: false,
            bg_color: [0.; 3],
            sel_color: Color::rgb(75, 75, 75),
            cursor_color: Color::rgb(160, 160, 160),
            cursor_active_color: Color::WHITE,
        }
    }
}
