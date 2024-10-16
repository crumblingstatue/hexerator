use {
    super::{WinCtx, WindowOpen},
    crate::{
        app::command::Cmd, gui::windows::regions_window::region_context_menu, meta::PerspectiveKey,
    },
    egui_extras::{Column, TableBuilder},
    slotmap::Key,
};

#[derive(Default)]
pub struct PerspectivesWindow {
    pub open: WindowOpen,
    pub rename_idx: PerspectiveKey,
}
impl super::Window for PerspectivesWindow {
    fn ui(&mut self, WinCtx { ui, gui, app, .. }: WinCtx) {
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
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
                        if self.rename_idx == keys[idx] {
                            let re = ui.text_edit_singleline(
                                &mut app.meta_state.meta.low.perspectives[keys[idx]].name,
                            );
                            if re.lost_focus() {
                                self.rename_idx = PerspectiveKey::null();
                            } else {
                                re.request_focus();
                            }
                        } else {
                            let name = &app.meta_state.meta.low.perspectives[keys[idx]].name;
                            ui.menu_button(name, |ui| {
                                if ui.button("✏ Rename").clicked() {
                                    self.rename_idx = keys[idx];
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
                                    ui.close_menu();
                                }
                                ui.menu_button("Containing views", |ui| {
                                    for (view_key, view) in app.meta_state.meta.views.iter() {
                                        if view.view.perspective == keys[idx]
                                            && ui.button(&view.name).clicked()
                                        {
                                            gui.win.views.open.set(true);
                                            gui.win.views.selected = view_key;
                                            ui.close_menu();
                                        }
                                    }
                                });
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
                            );
                        });
                        if re.clicked() {
                            gui.win.regions.open.set(true);
                            gui.win.regions.selected_key = Some(per.region);
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

    fn title(&self) -> &str {
        "Perspectives"
    }
}
