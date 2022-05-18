use std::ffi::OsString;

/// The hexerator application state
pub struct App {
    pub rows: usize,
    // Number of columns in the view
    pub cols: usize,
    // Maximum number of visible hex columns that can be shown on screen.
    // ascii is double this amount.
    pub max_visible_cols: usize,
    /// Path to the file we're editing
    pub path: OsString,
}

impl App {
    pub fn new(path: OsString) -> Self {
        Self {
            rows: 67,
            cols: 48,
            max_visible_cols: 75,
            path,
        }
    }
}
