use std::fs::OpenOptions;

use egui_sfml::sfml::graphics::Font;

use crate::app::App;

pub fn open_file(app: &mut App, font: &Font) {
    if let Some(path) = rfd::FileDialog::new().pick_file() {
        let write = OpenOptions::new().write(true).open(&path).is_ok();
        msg_if_fail(
            app.load_file(path, !write, font),
            "Failed to load file (read-write)",
        );
    }
}

pub fn open_previous(app: &mut App, load: &mut Option<crate::args::SourceArgs>) {
    if let Some(src_args) = app.cfg.recent.iter().nth(1) {
        *load = Some(src_args.clone());
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
