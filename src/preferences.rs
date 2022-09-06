#[derive(Debug)]
pub struct Preferences {
    /// Move the edit cursor with the cursor keys, instead of block cursor
    pub move_edit_cursor: bool,
    /// Immediately apply changes when editing a value, instead of having
    /// to type everything or press enter
    pub quick_edit: bool,
    /// Don't move the cursor after editing is finished
    pub sticky_edit: bool,
    /// Automatically save when editing is finished
    pub auto_save: bool,
    /// Keep metadata when loading.
    pub keep_meta: bool,
    /// Try to stay on current column when changing column count
    pub col_change_lock_col: bool,
    /// Try to stay on current row when changing column count
    pub col_change_lock_row: bool,
    /// Background color (mostly for fun)
    pub bg_color: [f32; 3],
    /// If true, auto-reload the current file at specified interval
    pub auto_reload: bool,
    /// Auto-reload interval in milliseconds
    pub auto_reload_interval_ms: u32,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            move_edit_cursor: false,
            quick_edit: false,
            sticky_edit: false,
            auto_save: false,
            keep_meta: false,
            col_change_lock_col: false,
            col_change_lock_row: true,
            bg_color: [0.0; 3],
            auto_reload: false,
            auto_reload_interval_ms: 250,
        }
    }
}
