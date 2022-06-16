use std::collections::HashSet;

use egui_inspect::{derive::Inspect, UiExt};
use egui_sfml::egui;
use sfml::window::{Event, Key};

#[derive(Default, Inspect, Debug)]
pub struct Input {
    #[inspect_with(inspect_key_down)]
    key_down: HashSet<Key>,
}

fn inspect_key_down(keys: &mut HashSet<Key>, ui: &mut egui::Ui, mut id_source: u64) {
    ui.inspect_iter_with("HashSet", keys.iter(), &mut id_source, inspect_key);
}

fn inspect_key(ui: &mut egui::Ui, i: usize, key: &Key, _id_source: &mut u64) {
    ui.label(format!("({}) {:?}", i, key));
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
