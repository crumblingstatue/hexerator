use crate::meta::region::Region;

#[derive(Default, Debug)]
pub struct EditState {
    // The editing byte offset
    pub cursor: usize,
    cursor_history: Vec<usize>,
    cursor_history_current: usize,
    pub dirty_region: Option<Region>,
}

impl EditState {
    /// Set cursor and save history
    pub fn set_cursor(&mut self, offset: usize) {
        self.cursor_history.truncate(self.cursor_history_current);
        self.cursor_history.push(self.cursor);
        self.cursor = offset;
        self.cursor_history_current += 1;
    }
    /// Set cursor, don't save history
    pub fn set_cursor_no_history(&mut self, offset: usize) {
        self.cursor = offset;
    }
    /// Step cursor forward without saving history
    pub fn step_cursor_forward(&mut self) {
        self.cursor += 1;
    }
    /// Step cursor back without saving history
    pub fn step_cursor_back(&mut self) {
        self.cursor = self.cursor.saturating_sub(1)
    }
    /// Offset cursor by amount, not saving history
    pub fn offset_cursor(&mut self, amount: usize) {
        self.cursor += amount;
    }
    pub fn cursor_history_back(&mut self) -> bool {
        if self.cursor_history_current > 0 {
            self.cursor_history.push(self.cursor);
            self.cursor_history_current -= 1;
            self.cursor = self.cursor_history[self.cursor_history_current];
            true
        } else {
            false
        }
    }
    pub fn cursor_history_forward(&mut self) -> bool {
        if self.cursor_history_current + 1 < self.cursor_history.len() {
            self.cursor_history_current += 1;
            self.cursor = self.cursor_history[self.cursor_history_current];
            true
        } else {
            false
        }
    }
}
