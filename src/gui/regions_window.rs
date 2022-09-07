use egui_extras::{Size, TableBuilder};
use egui_sfml::egui::{self, Ui};

use crate::{
    app::App,
    meta::{NamedRegion, RegionKey},
};

use super::window_open::WindowOpen;

#[derive(Default)]
pub struct RegionsWindow {
    pub open: WindowOpen,
    pub selected_key: Option<RegionKey>,
    select_active: bool,
    rename_active: bool,
}

#[macro_export]
macro_rules! region_context_menu {
    ($app:expr, $reg:expr, $action:expr) => {
        |ui: &mut egui_sfml::egui::Ui| {
            ui.menu_button("Containing layouts", |ui| {
                for (key, layout) in $app.meta_state.meta.layouts.iter() {
                    if let Some(v) =
                        layout.view_containing_region(&$reg.region, &$app.meta_state.meta)
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
            if ui.button("Select").clicked() {
                $app.hex_ui.select_a = Some($reg.region.begin);
                $app.hex_ui.select_b = Some($reg.region.end);
                ui.close_menu();
            }
        }
    };
}

impl RegionsWindow {
    pub fn ui(ui: &mut Ui, gui: &mut crate::gui::Gui, app: &mut App) {
        let button = egui::Button::new("Add selection as region");
        match App::selection(&app.hex_ui.select_a, &app.hex_ui.select_b) {
            Some(sel) => {
                if ui.add(button).clicked() {
                    app.meta_state.meta.regions.insert(NamedRegion {
                        name: String::from("<Unnamed>"),
                        region: sel,
                        desc: String::new(),
                    });
                    app.meta_state.meta_dirty = true;
                }
            }
            None => {
                ui.add_enabled(false, button);
            }
        }
        ui.separator();
        TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .column(Size::remainder().at_least(200.0))
            .column(Size::remainder().at_least(80.0))
            .column(Size::remainder().at_least(80.0))
            .column(Size::remainder().at_least(80.0))
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
                let mut keys: Vec<RegionKey> = app.meta_state.meta.regions.keys().collect();
                let mut action = Action::None;
                keys.sort_by_key(|k| app.meta_state.meta.regions[*k].region.begin);
                for k in keys {
                    body.row(20.0, |mut row| {
                        let reg = &app.meta_state.meta.regions[k];
                        row.col(|ui| {
                            let ctx_menu = region_context_menu!(app, reg, action);
                            if ui
                                .selectable_label(
                                    gui.regions_window.selected_key == Some(k),
                                    &reg.name,
                                )
                                .on_hover_text(&reg.desc)
                                .context_menu(ctx_menu)
                                .clicked()
                            {
                                gui.regions_window.selected_key = Some(k);
                            }
                        });
                        row.col(|ui| {
                            if ui.link(reg.region.begin.to_string()).clicked() {
                                action = Action::Goto(reg.region.begin);
                            }
                        });
                        row.col(|ui| {
                            if ui.link(reg.region.end.to_string()).clicked() {
                                action = Action::Goto(reg.region.end);
                            }
                        });
                        row.col(|ui| {
                            ui.label(((reg.region.end + 1) - reg.region.begin).to_string());
                        });
                    });
                }
                match action {
                    Action::None => {}
                    Action::Goto(off) => {
                        app.center_view_on_offset(off);
                        app.edit_state.set_cursor(off);
                        app.flash_cursor();
                    }
                }
            });
        ui.separator();
        if let &Some(key) = &gui.regions_window.selected_key {
            let reg = &mut app.meta_state.meta.regions[key];
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
                ui.add(egui::DragValue::new(&mut reg.region.begin));
                ui.label("Last byte");
                ui.add(egui::DragValue::new(&mut reg.region.end));
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
            if let Some(sel) = App::selection(&app.hex_ui.select_a, &app.hex_ui.select_b) {
                if ui.button("Set to selection").clicked() {
                    reg.region = sel;
                }
            } else {
                ui.add_enabled(false, egui::Button::new("Set to selection"));
            }
            ui.label("Description");
            ui.text_edit_multiline(&mut reg.desc);
            if ui.button("Delete").clicked() {
                app.meta_state.meta.regions.remove(key);
                gui.regions_window.selected_key = None;
            }
        }
    }
}

enum Action {
    None,
    Goto(usize),
}
