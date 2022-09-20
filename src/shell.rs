use {
    crate::{
        app::App,
        gui::message_dialog::{Icon, MessageDialog},
    },
    egui_sfml::sfml::graphics::Font,
    std::fs::OpenOptions,
};

pub fn open_file(app: &mut App, font: &Font, msg: &mut MessageDialog) {
    if let Some(path) = rfd::FileDialog::new().pick_file() {
        let write = OpenOptions::new().write(true).open(&path).is_ok();
        msg_if_fail(
            app.load_file(path, !write, font, msg),
            "Failed to load file (read-write)",
            msg,
        );
    }
}

pub fn open_previous(app: &mut App, load: &mut Option<crate::args::SourceArgs>) {
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
    msg.open(Icon::Error, "Error", format!("{}: {:?}", prefix, e));
}
