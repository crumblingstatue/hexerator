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
    pub col_width: u8,
    // The editing byte offset
    pub cursor: usize,
    // The byte offset in the data from which the view starts viewing data from
    pub starting_offset: usize,
}

pub enum CursorViewStatus {
    Inside,
    Before,
    After,
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
            col_width: 26,
            cursor: 0,
            starting_offset: 0,
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
    pub fn ascii_display_x_offset(&self) -> i64 {
        self.cols as i64 * i64::from(self.col_width) + 12
    }
    pub fn cursor_view_status(&self) -> CursorViewStatus {
        if self.cursor < self.starting_offset {
            CursorViewStatus::Before
        } else if self.cursor > self.starting_offset + self.rows * self.cols {
            CursorViewStatus::After
        } else {
            CursorViewStatus::Inside
        }
    }
    pub fn search_focus(&mut self, off: usize) {
        // Focus the search result in the hex view
        self.cursor = off;
        match self.cursor_view_status() {
            CursorViewStatus::Before => {
                self.starting_offset = off.saturating_sub((self.rows - 1) * (self.cols - 1))
            }
            CursorViewStatus::After => self.starting_offset = off - (self.rows + self.cols),
            CursorViewStatus::Inside => {}
        }
    }
}
