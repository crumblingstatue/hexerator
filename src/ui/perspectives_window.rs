use egui_sfml::egui;

use crate::meta::perspective::Perspective;

use super::window_open::WindowOpen;

#[derive(Default)]
pub struct PerspectivesWindow {
    pub open: WindowOpen,
    pub rename: bool,
}
impl PerspectivesWindow {
    pub(crate) fn ui(ui: &mut egui_sfml::egui::Ui, app: &mut crate::app::App) {
        app.meta.perspectives.retain(|k, per| {
            let mut retain = true;
            if app.ui.perspectives_window.rename {
                if ui.text_edit_singleline(&mut per.name).lost_focus() {
                    app.ui.perspectives_window.rename = false;
                }
            } else {
                ui.horizontal(|ui| {
                    ui.heading(&per.name);
                    if ui.button("✏").on_hover_text("Rename").clicked() {
                        app.ui.perspectives_window.rename ^= true;
                    }
                });
            }
            egui::ComboBox::new(egui::Id::new("region_combo").with(k), "region")
                .selected_text(&app.meta.regions[per.region].name)
                .show_ui(ui, |ui| {
                    for (reg_k, reg) in &app.meta.regions {
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
        ui.menu_button("New from region", |ui| {
            for (key, region) in app.meta.regions.iter() {
                if ui.button(&region.name).clicked() {
                    app.meta
                        .perspectives
                        .insert(Perspective::from_region(key, region.name.clone()));
                    return;
                }
            }
        });
    }
}
