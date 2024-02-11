use {
    super::{regions_window::region_context_menu, window_open::WindowOpen},
    crate::{app::command::Cmd, meta::PerspectiveKey},
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
                            let name = &app.meta_state.meta.low.perspectives[keys[idx]].name;
                            ui.menu_button(name, |ui| {
                                if ui.button("✏ Rename").clicked() {
                                    gui.perspectives_window.rename_idx = keys[idx];
                                    ui.close_menu();
                                }
                                if ui.button("🗑 Delete").clicked() {
                                    app.cmd.push(Cmd::RemovePerspective(keys[idx]));
                                    ui.close_menu();
                                }
                                if ui.button("Create view").clicked() {
                                    app.cmd.push(Cmd::CreateView {
                                        perspective_key: keys[idx],
                                        name: name.to_owned(),
                                    });
                                }
                            });
                        }
                    });
                    row.col(|ui| {
                        let per = &app.meta_state.meta.low.perspectives[keys[idx]];
                        let reg = &app.meta_state.meta.low.regions[per.region];
                        let re = ui.link(&reg.name).on_hover_text(&reg.desc);
                        re.context_menu(|ui| {
                            region_context_menu(
                                ui,
                                reg,
                                per.region,
                                &app.meta_state.meta,
                                &mut app.cmd,
                                &mut gui.cmd,
                            )
                        });
                        if re.clicked() {
                            gui.regions_window.open.set(true);
                            gui.regions_window.selected_key = Some(per.region);
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
                    app.cmd.push(Cmd::CreatePerspective {
                        region_key: key,
                        name: region.name.clone(),
                    });
                    ui.close_menu();
                    return;
                }
            }
        });
    }
}
