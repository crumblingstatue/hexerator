use std::collections::HashSet;

use sfml::window::{Event, Key};

#[derive(Default)]
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
}
