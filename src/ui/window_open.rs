#[derive(Default)]
pub struct WindowOpen {
    open: bool,
    just_opened: bool,
}

impl WindowOpen {
    pub fn toggle(&mut self) {
        self.open ^= true;
        if self.open {
            self.just_opened = true;
        }
    }
    pub fn is_open(&self) -> bool {
        self.open
    }
    pub fn set_open(&mut self, open: bool) {
        if !self.open && open {
            self.just_opened = true;
        }
        self.open = open;
    }
    pub fn just_opened(&self) -> bool {
        self.just_opened
    }
    /// Call this at the end of your ui, where you won't query just_opened anymore
    pub fn post_ui(&mut self) {
        self.just_opened = false;
    }
}
