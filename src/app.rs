/// The hexerator application state
pub struct App {
    pub rows: usize,
    // Number of columns in the view
    pub cols: usize,
    // Maximum number of visible hex columns that can be shown on screen.
    // ascii is double this amount.
    pub max_visible_cols: usize,
}

impl Default for App {
    fn default() -> Self {
        Self {
            rows: 67,
            cols: 48,
            max_visible_cols: 75,
        }
    }
}
