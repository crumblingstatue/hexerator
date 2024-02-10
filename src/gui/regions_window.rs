use {
    super::window_open::WindowOpen,
    crate::{app::App, meta::RegionKey},
    egui::{self, Ui},
    egui_extras::{Column, TableBuilder},
};

#[derive(Default)]
pub struct RegionsWindow {
    pub open: WindowOpen,
    pub selected_key: Option<RegionKey>,
    select_active: bool,
    rename_active: bool,
}

#[macro_export]
macro_rules! region_context_menu {
    ($ui:expr, $app:expr, $key:expr, $reg:expr, $action:expr) => {{
        $ui.menu_button("Containing layouts", |ui| {
            for (key, layout) in $app.meta_state.meta.layouts.iter() {
                if let Some(v) = layout.view_containing_region(&$reg.region, &$app.meta_state.meta)
                {
                    if ui.button(&layout.name).clicked() {
                        $app.hex_ui.current_layout = key;
                        $app.hex_ui.focused_view = Some(v);
                        $action = Action::Goto($reg.region.begin);
                        ui.close_menu();
                    }
                }
            }
        });
        if $ui.button("Select").clicked() {
            $app.cmd.push($crate::app::command::Cmd::SetSelection(
                $reg.region.begin,
                $reg.region.end,
            ));
            $ui.close_menu();
        }
        if $ui.button("Create perspective").clicked() {
            $app.cmd.push($crate::app::command::Cmd::CreatePerspective {
                region_key: $key,
                name: $reg.name.clone(),
            });
            $ui.close_menu();
        }
    }};
}

impl RegionsWindow {
    pub fn ui(ui: &mut Ui, gui: &mut crate::gui::Gui, app: &mut App) {
        let button = egui::Button::new("Add selection as region");
        match app.hex_ui.selection() {
            Some(sel) => {
                if ui.add(button).clicked() {
                    super::ops::add_region_from_selection(
                        sel,
                        &mut app.meta_state,
                        &mut gui.regions_window,
                    );
                }
            }
            None => {
                ui.add_enabled(false, button);
            }
        }
        if let &Some(key) = &gui.regions_window.selected_key {
            ui.separator();
            let reg = &mut app.meta_state.meta.low.regions[key];
            ui.horizontal(|ui| {
                if gui.regions_window.rename_active {
                    if ui.text_edit_singleline(&mut reg.name).lost_focus() {
                        gui.regions_window.rename_active = false;
                    }
                } else {
                    ui.heading(&reg.name);
                }
                if ui.button("✏").on_hover_text("Rename").clicked() {
                    gui.regions_window.rename_active ^= true;
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
            if gui.regions_window.select_active {
                app.hex_ui.select_a = Some(reg.region.begin);
                app.hex_ui.select_b = Some(reg.region.end);
            }
            if ui
                .checkbox(&mut gui.regions_window.select_active, "Select")
                .clicked()
            {
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
                gui.regions_window.selected_key = None;
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
            .body(|mut body| {
                let mut keys: Vec<RegionKey> = app.meta_state.meta.low.regions.keys().collect();
                let mut action = Action::None;
                keys.sort_by_key(|k| app.meta_state.meta.low.regions[*k].region.begin);
                for k in keys {
                    body.row(20.0, |mut row| {
                        let reg = &app.meta_state.meta.low.regions[k];
                        row.col(|ui| {
                            let ctx_menu =
                                |ui: &mut egui::Ui| region_context_menu!(ui, app, k, reg, action);
                            let re = ui
                                .selectable_label(
                                    gui.regions_window.selected_key == Some(k),
                                    &reg.name,
                                )
                                .on_hover_text(&reg.desc);
                            re.context_menu(ctx_menu);
                            if re.clicked() {
                                gui.regions_window.selected_key = Some(k);
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
                }
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
}

enum Action {
    None,
    Goto(usize),
    SetRegionBegin { key: RegionKey, begin: usize },
    SetRegionEnd { key: RegionKey, end: usize },
}
