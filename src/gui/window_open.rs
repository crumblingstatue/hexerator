#[derive(Default)]
pub struct WindowOpen {
    open: bool,
    just_opened: bool,
}

impl WindowOpen {
    /// Open if closed, close if opened
    pub fn toggle(&mut self) {
        self.open ^= true;
        if self.open {
            self.just_opened = true;
        }
    }
    /// Wheter the window is open
    pub fn is(&self) -> bool {
        self.open
    }
    /// Set whether the window is open
    pub fn set(&mut self, open: bool) {
        if !self.open && open {
            self.just_opened = true;
        }
        self.open = open;
    }
    /// Whether the window was opened just now (this frame)
    pub fn just_now(&self) -> bool {
        self.just_opened
    }
    /// Call this at the end of your ui, where you won't query just_opened anymore
    pub fn post_ui(&mut self) {
        self.just_opened = false;
    }
}
