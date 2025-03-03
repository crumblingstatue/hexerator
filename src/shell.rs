use {
    crate::{
        app::App,
        gui::message_dialog::{Icon, MessageDialog},
    },
    std::backtrace::Backtrace,
};

pub fn open_previous(app: &App, load: &mut Option<crate::args::SourceArgs>) {
    if let Some(src_args) = app.cfg.recent.iter().nth(1) {
        *load = Some(src_args.clone());
    }
}

pub fn msg_if_fail<T, E: std::fmt::Display>(
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

pub fn msg_fail<E: std::fmt::Display>(e: &E, prefix: &str, msg: &mut MessageDialog) {
    msg.open(Icon::Error, "Error", format!("{prefix}: {e:#}"));
    msg.backtrace = Some(Backtrace::force_capture());
}
