use {
    egui_sfml::egui::{self, Ui},
    gamedebug_core::{Info, PerEntry, IMMEDIATE, PERSISTENT},
};

pub fn ui(ui: &mut Ui) {
    match IMMEDIATE.lock() {
        Ok(imm) => {
            egui::ScrollArea::vertical()
                .max_height(500.)
                .show(ui, |ui| {
                    for info in imm.iter() {
                        if let Info::Msg(msg) = info {
                            ui.label(msg);
                        }
                    }
                });
        }
        Err(e) => {
            ui.label(&format!("IMMEDIATE lock fail: {}", e));
        }
    }
    gamedebug_core::clear_immediates();
    ui.separator();
    match PERSISTENT.lock() {
        Ok(per) => {
            egui::ScrollArea::vertical()
                .id_source("per_scroll")
                .max_height(500.0)
                .show(ui, |ui| {
                    for PerEntry { frame, info } in per.iter() {
                        if let Info::Msg(msg) = info {
                            ui.label(format!("{}: {}", frame, msg));
                        }
                    }
                });
        }
        Err(e) => {
            ui.label(&format!("PERSISTENT lock fail: {}", e));
        }
    }
}
