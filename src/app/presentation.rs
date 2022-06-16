use crate::color::ColorMethod;

#[derive(Debug)]
pub struct Presentation {
    pub color_method: ColorMethod,
    pub invert_color: bool,
    pub bg_color: [f32; 3],
}

impl Default for Presentation {
    fn default() -> Self {
        Self {
            color_method: ColorMethod::Default,
            invert_color: false,
            bg_color: [0.; 3],
        }
    }
}
