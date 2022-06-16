use egui_sfml::egui::Ui;
use gamedebug_core::{Info, PerEntry, IMMEDIATE, PERSISTENT};

#[expect(
    clippy::significant_drop_in_scrutinee,
    reason = "this isn't a useful lint for for loops"
)]
// https://github.com/rust-lang/rust-clippy/issues/8987
pub fn ui(ui: &mut Ui) {
    for info in IMMEDIATE.lock().unwrap().iter() {
        if let Info::Msg(msg) = info {
            ui.label(msg);
        }
    }
    gamedebug_core::clear_immediates();
    ui.separator();
    for PerEntry { frame, info } in PERSISTENT.lock().unwrap().iter() {
        if let Info::Msg(msg) = info {
            ui.label(format!("{}: {}", frame, msg));
        }
    }
}
