use {
    super::{WinCtx, WindowOpen},
    crate::{
        app::command::{Cmd, CommandQueue},
        gui::command::{GCmd, GCommandQueue},
        meta::{Meta, NamedRegion, RegionKey},
    },
    egui_extras::{Column, TableBuilder},
};

#[derive(Default)]
pub struct RegionsWindow {
    pub open: WindowOpen,
    pub selected_key: Option<RegionKey>,
    select_active: bool,
    rename_active: bool,
}

pub fn region_context_menu(
    ui: &mut egui::Ui,
    reg: &NamedRegion,
    key: RegionKey,
    meta: &Meta,
    cmd: &mut CommandQueue,
    gcmd: &mut GCommandQueue,
) {
    ui.menu_button("Containing layouts", |ui| {
        for (key, layout) in meta.layouts.iter() {
            if let Some(v) = layout.view_containing_region(&reg.region, meta) {
                if ui.button(&layout.name).clicked() {
                    cmd.push(Cmd::SetLayout(key));
                    cmd.push(Cmd::FocusView(v));
                    cmd.push(Cmd::SetAndFocusCursor(reg.region.begin));
                    ui.close_menu();
                }
            }
        }
    });
    ui.menu_button("Containing perspectives", |ui| {
        for (_per_key, per) in meta.low.perspectives.iter() {
            if per.region == key && ui.button(&per.name).clicked() {
                gcmd.push(GCmd::OpenPerspectiveWindow);
            }
        }
    });
    if ui.button("Select").clicked() {
        cmd.push(Cmd::SetSelection(reg.region.begin, reg.region.end));
        ui.close_menu();
    }
    if ui.button("Create perspective").clicked() {
        cmd.push(Cmd::CreatePerspective {
            region_key: key,
            name: reg.name.clone(),
        });
        ui.close_menu();
    }
}

impl super::Window for RegionsWindow {
    fn ui(&mut self, WinCtx { ui, gui, app, .. }: WinCtx) {
        ui.style_mut().wrap = Some(false);
        let button = egui::Button::new("Add selection as region");
        match app.hex_ui.selection() {
            Some(sel) => {
                if ui.add(button).clicked() {
                    crate::gui::ops::add_region_from_selection(sel, &mut app.meta_state, self);
                }
            }
            None => {
                ui.add_enabled(false, button);
            }
        }
        if let &Some(key) = &self.selected_key {
            ui.separator();
            let reg = &mut app.meta_state.meta.low.regions[key];
            ui.horizontal(|ui| {
                if self.rename_active {
                    if ui.text_edit_singleline(&mut reg.name).lost_focus() {
                        self.rename_active = false;
                    }
                } else {
                    ui.heading(&reg.name);
                }
                if ui.button("✏").on_hover_text("Rename").clicked() {
                    self.rename_active ^= true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("First byte");
                ui.add(egui::DragValue::new(&mut reg.region.begin))
                    .context_menu(|ui| {
                        if ui.button("Set to cursor").clicked() {
                            reg.region.begin = app.edit_state.cursor;
                            ui.close_menu();
                        }
                    });
                ui.label("Last byte");
                ui.add(egui::DragValue::new(&mut reg.region.end))
                    .context_menu(|ui| {
                        if ui.button("Set to cursor").clicked() {
                            reg.region.end = app.edit_state.cursor;
                            ui.close_menu();
                        }
                    });
            });
            if self.select_active {
                app.hex_ui.select_a = Some(reg.region.begin);
                app.hex_ui.select_b = Some(reg.region.end);
            }
            if ui.checkbox(&mut self.select_active, "Select").clicked() {
                app.hex_ui.select_a = None;
                app.hex_ui.select_b = None;
            }
            if let Some(sel) = app.hex_ui.selection() {
                if ui.button("Set to selection").clicked() {
                    reg.region = sel;
                }
            } else {
                ui.add_enabled(false, egui::Button::new("Set to selection"));
            }
            if ui
                .button("Reset")
                .on_hover_text("Encompass the whole document")
                .clicked()
            {
                reg.region.begin = 0;
                reg.region.end = app.data.len() - 1;
            }
            ui.label("Description");
            ui.text_edit_multiline(&mut reg.desc);
            if ui.button("Delete").clicked() {
                app.meta_state.meta.low.regions.remove(key);
                app.remove_dangling();
                self.selected_key = None;
            }
        }
        ui.separator();
        TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .column(Column::auto())
            .column(Column::auto())
            .column(Column::auto())
            .column(Column::remainder())
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.label("Name");
                });
                header.col(|ui| {
                    ui.label("First byte");
                });
                header.col(|ui| {
                    ui.label("Last byte");
                });
                header.col(|ui| {
                    ui.label("Length");
                });
            })
            .body(|body| {
                let mut keys: Vec<RegionKey> = app.meta_state.meta.low.regions.keys().collect();
                let mut action = Action::None;
                keys.sort_by_key(|k| app.meta_state.meta.low.regions[*k].region.begin);
                body.rows(20.0, keys.len(), |mut row| {
                    let k = keys[row.index()];
                    let reg = &app.meta_state.meta.low.regions[k];
                    row.col(|ui| {
                        let ctx_menu = |ui: &mut egui::Ui| {
                            region_context_menu(
                                ui,
                                reg,
                                k,
                                &app.meta_state.meta,
                                &mut app.cmd,
                                &mut gui.cmd,
                            )
                        };
                        let re = ui
                            .selectable_label(self.selected_key == Some(k), &reg.name)
                            .on_hover_text(&reg.desc);
                        re.context_menu(ctx_menu);
                        if re.clicked() {
                            self.selected_key = Some(k);
                        }
                    });
                    row.col(|ui| {
                        let re = ui.link(reg.region.begin.to_string());
                        re.context_menu(|ui| {
                            if ui.button("Set to cursor").clicked() {
                                action = Action::SetRegionBegin {
                                    key: k,
                                    begin: app.edit_state.cursor,
                                };
                                ui.close_menu();
                            }
                        });
                        if re.clicked() {
                            action = Action::Goto(reg.region.begin);
                        }
                    });
                    row.col(|ui| {
                        let re = ui.link(reg.region.end.to_string());
                        re.context_menu(|ui| {
                            if ui.button("Set to cursor").clicked() {
                                action = Action::SetRegionEnd {
                                    key: k,
                                    end: app.edit_state.cursor,
                                };
                                ui.close_menu();
                            }
                        });
                        if re.clicked() {
                            action = Action::Goto(reg.region.end);
                        }
                    });
                    row.col(
                        |ui| match (reg.region.end + 1).checked_sub(reg.region.begin) {
                            Some(len) => {
                                ui.label(len.to_string());
                            }
                            None => {
                                ui.label("Overflow!");
                            }
                        },
                    );
                });
                match action {
                    Action::None => {}
                    Action::Goto(off) => {
                        app.center_view_on_offset(off);
                        app.edit_state.set_cursor(off);
                        app.hex_ui.flash_cursor();
                    }
                    Action::SetRegionBegin { key, begin } => {
                        app.meta_state.meta.low.regions[key].region.begin = begin
                    }
                    Action::SetRegionEnd { key, end } => {
                        app.meta_state.meta.low.regions[key].region.end = end
                    }
                }
            });
    }

    fn title(&self) -> &str {
        "Regions"
    }
}

enum Action {
    None,
    Goto(usize),
    SetRegionBegin { key: RegionKey, begin: usize },
    SetRegionEnd { key: RegionKey, end: usize },
}
