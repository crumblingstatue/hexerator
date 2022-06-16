#[derive(Debug)]
pub struct Layout {
    pub top_gap: i64,
    pub bottom_gap: i64,
    pub window_height: u32,
    pub row_height: u8,
    pub col_width: u8,
    // Maximum number of visible hex columns that can be shown on screen.
    // ascii is double this amount.
    pub max_visible_cols: usize,
    /// Font size
    pub font_size: u32,
    /// Block size for block view
    pub block_size: u8,
}

impl Layout {
    pub fn new(window_height: u32) -> Self {
        Self {
            font_size: 14,
            block_size: 4,
            max_visible_cols: 75,
            col_width: 26,
            top_gap: 46,
            row_height: 16,
            bottom_gap: 25,
            window_height,
        }
    }
}
