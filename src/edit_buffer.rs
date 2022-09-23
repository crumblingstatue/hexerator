use gamedebug_core::per;

#[derive(Debug, Default, Clone)]
pub struct EditBuffer {
    pub buf: Vec<u8>,
    pub cursor: u16,
    /// Whether this edit buffer has been edited
    pub dirty: bool,
}

impl EditBuffer {
    pub(crate) fn resize(&mut self, new_size: u16) {
        self.buf.resize(usize::from(new_size), 0);
    }
    /// Enter a byte. Returns if editing is "finished" (at end)
    pub(crate) fn enter_byte(&mut self, byte: u8) -> bool {
        self.dirty = true;
        self.buf[self.cursor as usize] = byte;
        self.cursor += 1;
        if usize::from(self.cursor) >= self.buf.len() {
            self.reset();
            true
        } else {
            false
        }
    }

    pub fn reset(&mut self) {
        self.cursor = 0;
        self.dirty = false;
    }

    pub(crate) fn update_from_string(&mut self, s: &str) {
        let bytes = s.as_bytes();
        self.buf[..bytes.len()].copy_from_slice(bytes);
    }
    /// Returns whether the cursor could be moved any further
    pub(crate) fn move_cursor_back(&mut self) -> bool {
        if self.cursor == 0 {
            false
        } else {
            self.cursor -= 1;
            true
        }
    }
    /// Move the cursor to the end
    #[expect(
        clippy::cast_possible_truncation,
        reason = "Buffer is never bigger than u16::MAX"
    )]
    pub(crate) fn move_cursor_end(&mut self) {
        self.cursor = (self.buf.len() - 1) as u16;
    }

    /// Returns whether the cursor could be moved any further
    #[expect(
        clippy::cast_possible_truncation,
        reason = "Buffer is never bigger than u16::MAX"
    )]
    pub(crate) fn move_cursor_forward(&mut self) -> bool {
        if self.cursor >= self.buf.len() as u16 - 1 {
            false
        } else {
            per!("Moving cursor forward, no problem");
            self.cursor += 1;
            true
        }
    }

    pub(crate) fn move_cursor_begin(&mut self) {
        self.cursor = 0;
    }
}
