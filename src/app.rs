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
    pub dirty: bool,
    pub data: Vec<u8>,
    pub show_debug_panel: bool,
}

impl App {
    pub fn new(path: OsString) -> Self {
        let data = std::fs::read(&path).unwrap();
        Self {
            rows: 67,
            cols: 48,
            max_visible_cols: 75,
            path,
            dirty: false,
            data,
            show_debug_panel: false,
        }
    }
    pub fn reload(&mut self) {
        self.data = std::fs::read(&self.path).unwrap();
        self.dirty = false;
    }
    pub fn save(&mut self) {
        std::fs::write(&self.path, &self.data).unwrap();
        self.dirty = false;
    }
    pub fn toggle_debug(&mut self) {
        self.show_debug_panel ^= true;
        gamedebug_core::toggle();
    }
}
