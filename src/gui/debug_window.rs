use {
    egui::Ui,
    gamedebug_core::{IMMEDIATE, PERSISTENT},
};

pub fn ui(ui: &mut Ui) {
    egui::ScrollArea::vertical()
        .max_height(500.)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            IMMEDIATE.for_each(|msg| {
                ui.label(msg);
            })
        });
    IMMEDIATE.clear();
    ui.separator();
    egui::ScrollArea::vertical()
        .id_source("per_scroll")
        .max_height(500.0)
        .show(ui, |ui| {
            PERSISTENT.for_each(|msg| {
                ui.label(format!("{}: {}", msg.frame, msg.info));
            });
        });
}
