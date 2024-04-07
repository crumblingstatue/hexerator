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
                    ui.label(
                        egui::RichText::new(msg.frame.to_string()).color(egui::Color32::DARK_GRAY),
                    );
                    if let Some(src_loc) = &msg.src_loc {
                        let txt = format!("{}:{}:{}", src_loc.file, src_loc.line, src_loc.column);
                        if ui
                            .link(&txt)
                            .on_hover_text("Click to copy to clipboard")
                            .clicked()
                        {
                            ui.output_mut(|out| out.copied_text = txt);
                        }
                    }
                    ui.label(&msg.info);
                    ui.end_row();
                });
            });
        });
}
