#[derive(Debug)]
pub struct Layout {
    pub top_gap: i16,
    pub bottom_gap: i16,
    pub window_height: u32,
    // Maximum number of visible hex columns that can be shown on screen.
    // ascii is double this amount.
    pub max_visible_cols: usize,
    /// Font size
    pub font_size: u8,
}

impl Layout {
    pub fn new(window_height: u32) -> Self {
        Self {
            font_size: 14,
            max_visible_cols: 75,
            top_gap: 46,
            bottom_gap: 25,
            window_height,
        }
    }
}
