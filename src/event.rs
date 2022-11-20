use {
    crate::app::App,
    egui_sfml::sfml::graphics::RenderWindow,
    gamedebug_core::per,
    std::{collections::VecDeque, path::Path},
};

/// An event that happened in Hexerator
///
/// Events are pushed to the event queue, and handled by the event handler function
#[derive(Debug)]
pub enum Event {
    SourceChanged,
}

pub type EventQueue = VecDeque<Event>;

fn path_filename_as_str(path: &Path) -> &str {
    path.file_name()
        .map_or("<no_filename>", |osstr| osstr.to_str().unwrap_or_default())
}

pub fn handle_events(events: &mut EventQueue, app: &mut App, window: &mut RenderWindow) {
    while let Some(event) = events.pop_front() {
        per!("Incoming event: {event:?}");
        match event {
            Event::SourceChanged => window.set_title(&format!(
                "{} - Hexerator",
                app.source_file().map_or("no source", path_filename_as_str)
            )),
        }
    }
}
