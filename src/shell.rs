use sfml::graphics::Font;

use crate::app::App;

pub fn open_file(app: &mut App, font: &Font) {
    if let Some(file) = rfd::FileDialog::new().pick_file() {
        msg_if_fail(
            app.load_file(file, false, font),
            "Failed to load file (read-write)",
        );
    }
}

pub fn msg_if_fail<T, E: std::fmt::Debug>(result: Result<T, E>, prefix: &str) -> Option<E> {
    if let Err(e) = result {
        msg_fail(&e, prefix);
        Some(e)
    } else {
        None
    }
}

pub fn msg_fail<E: std::fmt::Debug>(e: &E, prefix: &str) {
    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Error)
        .set_title("Error")
        .set_description(&format!("{}: {:?}", prefix, e))
        .show();
}

pub fn msg_warn(msg: &str) {
    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Warning)
        .set_title("Warning")
        .set_description(msg)
        .show();
}

pub fn msg_info(msg: &str) {
    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Info)
        .set_title("Info")
        .set_description(msg)
        .show();
}
