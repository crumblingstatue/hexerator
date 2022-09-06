use egui_extras::{Size, TableBuilder};
use egui_sfml::egui;
use slotmap::Key;

use crate::{
    meta::{perspective::Perspective, PerspectiveKey, RegionKey},
    region_context_menu,
};

use super::window_open::WindowOpen;

#[derive(Default)]
pub struct PerspectivesWindow {
    pub open: WindowOpen,
    pub rename_idx: PerspectiveKey,
}
impl PerspectivesWindow {
    pub(crate) fn ui(ui: &mut egui::Ui, gui: &mut crate::gui::Gui, app: &mut crate::app::App) {
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
                let keys: Vec<_> = app.meta_state.meta.perspectives.keys().collect();
                let mut action = Action::None;
                body.rows(20.0, keys.len(), |idx, mut row| {
                    row.col(|ui| {
                        if gui.perspectives_window.rename_idx == keys[idx] {
                            let re = ui.text_edit_singleline(
                                &mut app.meta_state.meta.perspectives[keys[idx]].name,
                            );
                            if re.lost_focus() {
                                gui.perspectives_window.rename_idx = PerspectiveKey::null();
                            } else {
                                re.request_focus();
                            }
                        } else {
                            ui.menu_button(
                                &app.meta_state.meta.perspectives[keys[idx]].name,
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
                        let per = &app.meta_state.meta.perspectives[keys[idx]];
                        let reg = &app.meta_state.meta.regions[per.region];
                        if ui
                            .link(&reg.name)
                            .on_hover_text(&reg.desc)
                            .context_menu(region_context_menu!(app, reg, action))
                            .clicked()
                        {
                            action = Action::OpenRegion(per.region);
                        }
                    });
                    row.col(|ui| {
                        ui.add(egui::DragValue::new(
                            &mut app.meta_state.meta.perspectives[keys[idx]].cols,
                        ));
                    });
                    row.col(|ui| {
                        ui.checkbox(
                            &mut app.meta_state.meta.perspectives[keys[idx]].flip_row_order,
                            "",
                        );
                    });
                });
                match action {
                    Action::None => {}
                    Action::Remove(key) => {
                        app.meta_state.meta.perspectives.remove(key);
                    }
                    Action::OpenRegion(key) => {
                        gui.regions_window.open = true;
                        gui.regions_window.selected_key = Some(key);
                    }
                    Action::Goto(off) => {
                        app.center_view_on_offset(off);
                        app.edit_state.set_cursor(off);
                        app.flash_cursor();
                    }
                }
            });
        ui.separator();
        ui.menu_button("New from region", |ui| {
            for (key, region) in app.meta_state.meta.regions.iter() {
                if ui.button(&region.name).clicked() {
                    app.meta_state
                        .meta
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
    Goto(usize),
}
