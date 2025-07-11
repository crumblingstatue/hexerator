use {
    super::{WinCtx, WindowOpen},
    crate::{
        app::command::Cmd, gui::windows::regions::region_context_menu, meta::PerspectiveKey,
        shell::msg_if_fail,
    },
    egui_extras::{Column, TableBuilder},
    slotmap::Key as _,
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
            .columns(Column::auto(), 4)
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
                    ui.label("Columns");
                });
                row.col(|ui| {
                    ui.label("Rows");
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
                                }
                                if ui.button("🗑 Delete").clicked() {
                                    app.cmd.push(Cmd::RemovePerspective(keys[idx]));
                                }
                                if ui.button("Create view").clicked() {
                                    app.cmd.push(Cmd::CreateView {
                                        perspective_key: keys[idx],
                                        name: name.to_owned(),
                                    });
                                }
                                ui.menu_button("Containing views", |ui| {
                                    for (view_key, view) in app.meta_state.meta.views.iter() {
                                        if view.view.perspective == keys[idx]
                                            && ui.button(&view.name).clicked()
                                        {
                                            gui.win.views.open.set(true);
                                            gui.win.views.selected = view_key;
                                        }
                                    }
                                });
                                if ui.button("Copy name to clipboard").clicked() {
                                    let res = app.clipboard.set_text(name);
                                    msg_if_fail(
                                        res,
                                        "Failed to copy to clipboard",
                                        &mut gui.msg_dialog,
                                    );
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
                            );
                        });
                        if re.clicked() {
                            gui.win.regions.open.set(true);
                            gui.win.regions.selected_key = Some(per.region);
                        }
                    });
                    row.col(|ui| {
                        let per = &mut app.meta_state.meta.low.perspectives[keys[idx]];
                        let reg = &app.meta_state.meta.low.regions[per.region];
                        ui.add(egui::DragValue::new(&mut per.cols).range(1..=reg.region.len()));
                    });
                    row.col(|ui| {
                        let per = &app.meta_state.meta.low.perspectives[keys[idx]];
                        let reg = &app.meta_state.meta.low.regions[per.region];
                        let reg_len = reg.region.len();
                        let cols = per.cols;
                        let rows = reg_len / cols;
                        let rem = reg_len % cols;
                        let rem_str: &str = if rem == 0 {
                            ""
                        } else {
                            &format!(" (rem: {rem})")
                        };
                        ui.label(format!("{rows}{rem_str}"));
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

                    return;
                }
            }
        });
    }

    fn title(&self) -> &str {
        "Perspectives"
    }
}
