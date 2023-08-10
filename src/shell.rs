use {
    crate::{
        app::App,
        event::EventQueue,
        gui::message_dialog::{Icon, MessageDialog},
    },
    egui_sfml::sfml::graphics::Font,
    std::{fs::OpenOptions, path::Path},
};

pub fn open_dialog_same_dir(src_path: Option<&Path>) -> rfd::FileDialog {
    let mut file_dialog = rfd::FileDialog::new();
    if let Some(src_path) = src_path && let Some(parent) = src_path.parent() {
        file_dialog = file_dialog.set_directory(parent);
    }
    file_dialog
}

pub fn open_file(app: &mut App, font: &Font, msg: &mut MessageDialog, events: &EventQueue) {
    if let Some(path) = open_dialog_same_dir(app.source_file()).pick_file() {
        let write = OpenOptions::new().write(true).open(&path).is_ok();
        msg_if_fail(
            app.load_file(path, !write, font, msg, events),
            "Failed to load file (read-write)",
            msg,
        );
    }
}

pub fn open_previous(app: &App, load: &mut Option<crate::args::SourceArgs>) {
    if let Some(src_args) = app.cfg.recent.iter().nth(1) {
        *load = Some(src_args.clone());
    }
}

pub fn msg_if_fail<T, E: std::fmt::Debug>(
    result: Result<T, E>,
    prefix: &str,
    msg: &mut MessageDialog,
) -> Option<E> {
    if let Err(e) = result {
        msg_fail(&e, prefix, msg);
        Some(e)
    } else {
        None
    }
}

pub fn msg_fail<E: std::fmt::Debug>(e: &E, prefix: &str, msg: &mut MessageDialog) {
    msg.open(Icon::Error, "Error", format!("{prefix}: {e:?}"));
}
