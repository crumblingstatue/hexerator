#[derive(Debug)]
pub struct Layout {
    pub top_gap: i16,
    pub bottom_gap: i16,
    /// Font size
    pub font_size: u8,
}

impl Layout {
    pub fn new() -> Self {
        Self {
            font_size: 14,
            top_gap: 46,
            bottom_gap: 25,
        }
    }
}
