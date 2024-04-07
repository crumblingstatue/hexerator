use {
    egui::Ui,
    gamedebug_core::{IMMEDIATE, PERSISTENT},
};

pub fn ui(ui: &mut Ui) {
    ui.horizontal(|ui| {
        if ui.button("Clear persistent").clicked() {
            PERSISTENT.clear();
        }
    });
    ui.separator();
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
            egui::Grid::new("per_grid").striped(true).show(ui, |ui| {
                PERSISTENT.for_each(|msg| {
                    ui.label(format!("{}: {}", msg.frame, msg.info));
                    ui.end_row();
                });
            });
        });
}
