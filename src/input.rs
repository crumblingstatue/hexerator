use std::collections::HashSet;

use egui_sfml::sfml::window::{Event, Key};

#[derive(Default, Debug)]
pub struct Input {
    key_down: HashSet<Key>,
}

impl Input {
    pub fn update_from_event(&mut self, event: &Event) {
        match event {
            Event::KeyPressed { code, .. } => {
                self.key_down.insert(*code);
            }
            Event::KeyReleased { code, .. } => {
                self.key_down.remove(code);
            }
            _ => {}
        }
    }
    pub fn key_down(&self, key: Key) -> bool {
        self.key_down.contains(&key)
    }

    pub(crate) fn clear(&mut self) {
        self.key_down.clear();
    }
}
