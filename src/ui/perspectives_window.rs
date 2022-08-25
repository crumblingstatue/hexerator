use egui_sfml::egui;
use slotmap::Key;

use crate::app::perspective::Perspective;

use super::window_open::WindowOpen;

#[derive(Default)]
pub struct PerspectivesWindow {
    pub open: WindowOpen,
}
impl PerspectivesWindow {
    pub(crate) fn ui(ui: &mut egui_sfml::egui::Ui, app: &mut crate::app::App) {
        app.perspectives.retain(|k, per| {
            let mut retain = true;
            let (heading, sel_text) = if per.region.is_null() {
                ("<null perspective>".to_string(), "<null>")
            } else {
                let name = &app.regions[per.region].name;
                (format!("{}:{}", name, per.cols), name.as_str())
            };
            ui.heading(heading);
            egui::ComboBox::new(egui::Id::new("region_combo").with(k), "region")
                .selected_text(sel_text)
                .show_ui(ui, |ui| {
                    for (reg_k, reg) in &app.regions {
                        ui.selectable_value(&mut per.region, reg_k, &reg.name);
                    }
                });
            ui.horizontal(|ui| {
                ui.label("column count");
                ui.add(egui::DragValue::new(&mut per.cols));
            });
            if ui.button("Delete").clicked() {
                retain = false;
            }
            ui.separator();
            retain
        });
        ui.separator();
        if ui.button("Add new").clicked() {
            app.perspectives.insert(Perspective::default());
        }
    }
}
