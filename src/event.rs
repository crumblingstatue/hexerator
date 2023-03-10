use {
    crate::{app::App, gui::top_menu::edit::EditMenuEvt},
    egui_sfml::sfml::graphics::RenderWindow,
    gamedebug_core::per,
    parking_lot::Mutex,
    std::{collections::VecDeque, path::Path, sync::Arc},
};

/// An event that happened in Hexerator
///
/// Events are pushed to the event queue, and handled by the event handler function
#[derive(Debug)]
pub enum Event {
    SourceChanged,
    EditMenuEvt(EditMenuEvt),
    QuitRequested,
}

pub type EventQueue = Arc<Mutex<VecDeque<Event>>>;

fn path_filename_as_str(path: &Path) -> &str {
    path.file_name()
        .map_or("<no_filename>", |osstr| osstr.to_str().unwrap_or_default())
}

/// Returns false if application should quit
pub fn handle_events(events: &EventQueue, app: &mut App, window: &mut RenderWindow) -> bool {
    let mut events = events.lock();
    while let Some(event) = events.pop_front() {
        per!("Incoming event: {event:?}");
        match event {
            Event::SourceChanged => window.set_title(&format!(
                "{} - Hexerator",
                app.source_file().map_or("no source", path_filename_as_str)
            )),
            Event::EditMenuEvt(ev) => match ev {
                EditMenuEvt::ExtendDocument { new_len } => {
                    app.data.resize(new_len, 0);
                }
                EditMenuEvt::PasteBytes { at, bytes } => {
                    app.data[at..at + bytes.len()].copy_from_slice(&bytes);
                }
            },
            Event::QuitRequested => return false,
        }
    }
    true
}
