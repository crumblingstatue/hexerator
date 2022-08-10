use sfml::graphics::Font;

use crate::{app::App, msg_if_fail};

pub fn open_file(app: &mut App, window_height: i16, font: &Font) {
    if let Some(file) = rfd::FileDialog::new().pick_file() {
        msg_if_fail(
            app.load_file(file, false, window_height, font),
            "Failed to load file (read-write)",
        );
    }
}
