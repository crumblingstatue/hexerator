use egui_sfml::egui::Ui;
use gamedebug_core::{Info, PerEntry, IMMEDIATE, PERSISTENT};

#[expect(
    clippy::significant_drop_in_scrutinee,
    reason = "this isn't a useful lint for for loops"
)]
// https://github.com/rust-lang/rust-clippy/issues/8987
pub fn ui(ui: &mut Ui) {
    match IMMEDIATE.lock() {
        Ok(imm) => {
            for info in imm.iter() {
                if let Info::Msg(msg) = info {
                    ui.label(msg);
                }
            }
        }
        Err(e) => {
            ui.label(&format!("IMMEDIATE lock fail: {}", e));
        }
    }
    gamedebug_core::clear_immediates();
    ui.separator();
    match PERSISTENT.lock() {
        Ok(per) => {
            for PerEntry { frame, info } in per.iter() {
                if let Info::Msg(msg) = info {
                    ui.label(format!("{}: {}", frame, msg));
                }
            }
        }
        Err(e) => {
            ui.label(&format!("PERSISTENT lock fail: {}", e));
        }
    }
}
