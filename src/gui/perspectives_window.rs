use {
    super::window_open::WindowOpen,
    crate::{
        meta::{PerspectiveKey, RegionKey},
        region_context_menu,
    },
    egui_extras::{Column, TableBuilder},
    slotmap::Key,
};

#[derive(Default)]
pub struct PerspectivesWindow {
    pub open: WindowOpen,
    pub rename_idx: PerspectiveKey,
}
impl PerspectivesWindow {
    pub(crate) fn ui(ui: &mut egui::Ui, gui: &mut crate::gui::Gui, app: &mut crate::app::App) {
        let mut action = Action::None;
        TableBuilder::new(ui)
            .columns(Column::auto(), 3)
            .column(Column::remainder())
            .striped(true)
            .resizable(true)
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
                let keys: Vec<_> = app.meta_state.meta.low.perspectives.keys().collect();
                body.rows(20.0, keys.len(), |mut row| {
                    let idx = row.index();
                    row.col(|ui| {
                        if gui.perspectives_window.rename_idx == keys[idx] {
                            let re = ui.text_edit_singleline(
                                &mut app.meta_state.meta.low.perspectives[keys[idx]].name,
                            );
                            if re.lost_focus() {
                                gui.perspectives_window.rename_idx = PerspectiveKey::null();
                            } else {
                                re.request_focus();
                            }
                        } else {
                            ui.menu_button(
                                &app.meta_state.meta.low.perspectives[keys[idx]].name,
                                |ui| {
                                    if ui.button("✏ Rename").clicked() {
                                        gui.perspectives_window.rename_idx = keys[idx];
                                        ui.close_menu();
                                    }
                                    if ui.button("🗑 Delete").clicked() {
                                        action = Action::Remove(keys[idx]);
                                        ui.close_menu();
                                    }
                                },
                            );
                        }
                    });
                    row.col(|ui| {
                        let per = &app.meta_state.meta.low.perspectives[keys[idx]];
                        let reg = &app.meta_state.meta.low.regions[per.region];
                        let re = ui.link(&reg.name).on_hover_text(&reg.desc);
                        re.context_menu(|ui| region_context_menu!(ui, app, per.region, reg));
                        if re.clicked() {
                            action = Action::OpenRegion(per.region);
                        }
                    });
                    row.col(|ui| {
                        ui.add(egui::DragValue::new(
                            &mut app.meta_state.meta.low.perspectives[keys[idx]].cols,
                        ));
                    });
                    row.col(|ui| {
                        ui.checkbox(
                            &mut app.meta_state.meta.low.perspectives[keys[idx]].flip_row_order,
                            "",
                        );
                    });
                });
            });
        ui.separator();
        ui.menu_button("New from region", |ui| {
            for (key, region) in app.meta_state.meta.low.regions.iter() {
                if ui.button(&region.name).clicked() {
                    action = Action::CreatePerspective {
                        region_key: key,
                        name: region.name.clone(),
                    };
                    ui.close_menu();
                    return;
                }
            }
        });
        match action {
            Action::None => {}
            Action::Remove(key) => {
                app.meta_state.meta.low.perspectives.remove(key);
            }
            Action::OpenRegion(key) => {
                gui.regions_window.open.set(true);
                gui.regions_window.selected_key = Some(key);
            }
            Action::CreatePerspective { region_key, name } => {
                app.add_perspective_from_region(region_key, name);
            }
        }
    }
}

enum Action {
    None,
    Remove(PerspectiveKey),
    OpenRegion(RegionKey),
    CreatePerspective { region_key: RegionKey, name: String },
}
