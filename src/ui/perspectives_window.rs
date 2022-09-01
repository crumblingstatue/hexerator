use egui_extras::{Size, TableBuilder};
use egui_sfml::egui;
use slotmap::Key;

use crate::meta::{perspective::Perspective, PerspectiveKey, RegionKey};

use super::window_open::WindowOpen;

#[derive(Default)]
pub struct PerspectivesWindow {
    pub open: WindowOpen,
    pub rename_idx: PerspectiveKey,
}
impl PerspectivesWindow {
    pub(crate) fn ui(ui: &mut egui::Ui, app: &mut crate::app::App) {
        TableBuilder::new(ui)
            .columns(Size::remainder(), 4)
            .striped(true)
            .header(24.0, |mut row| {
                row.col(|ui| {
                    ui.label("Name");
                });
                row.col(|ui| {
                    ui.label("Region");
                });
                row.col(|ui| {
                    ui.label("Column count");
                });
                row.col(|ui| {
                    ui.label("Flip row order");
                });
            })
            .body(|body| {
                let keys: Vec<_> = app.meta.perspectives.keys().collect();
                let mut action = Action::None;
                body.rows(20.0, keys.len(), |idx, mut row| {
                    let per = &mut app.meta.perspectives[keys[idx]];
                    row.col(|ui| {
                        if app.ui.perspectives_window.rename_idx == keys[idx] {
                            let re = ui.text_edit_singleline(&mut per.name);
                            if re.lost_focus() {
                                app.ui.perspectives_window.rename_idx = PerspectiveKey::null();
                            } else {
                                re.request_focus();
                            }
                        } else {
                            ui.menu_button(&per.name, |ui| {
                                if ui.button("✏ Rename").clicked() {
                                    app.ui.perspectives_window.rename_idx = keys[idx];
                                    ui.close_menu();
                                }
                                if ui.button("🗑 Delete").clicked() {
                                    action = Action::Remove(keys[idx]);
                                    ui.close_menu();
                                }
                            });
                        }
                    });
                    row.col(|ui| {
                        if ui.link(&app.meta.regions[per.region].name).clicked() {
                            action = Action::OpenRegion(per.region);
                        }
                    });
                    row.col(|ui| {
                        ui.add(egui::DragValue::new(&mut per.cols));
                    });
                    row.col(|ui| {
                        ui.checkbox(&mut per.flip_row_order, "");
                    });
                });
                match action {
                    Action::None => {}
                    Action::Remove(key) => {
                        app.meta.perspectives.remove(key);
                    }
                    Action::OpenRegion(key) => {
                        app.ui.regions_window.open = true;
                        app.ui.regions_window.selected_key = Some(key);
                    }
                }
            });
        ui.separator();
        ui.menu_button("New from region", |ui| {
            for (key, region) in app.meta.regions.iter() {
                if ui.button(&region.name).clicked() {
                    app.meta
                        .perspectives
                        .insert(Perspective::from_region(key, region.name.clone()));
                    ui.close_menu();
                    return;
                }
            }
        });
    }
}

enum Action {
    None,
    Remove(PerspectiveKey),
    OpenRegion(RegionKey),
}
