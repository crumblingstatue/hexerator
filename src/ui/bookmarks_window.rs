use egui_extras::{Size, TableBuilder};
use egui_sfml::egui::{self, Ui};

use crate::{
    app::App,
    meta::{find_most_specific_region_for_offset, Bookmark},
    region_context_menu,
};

use super::window_open::WindowOpen;

#[derive(Default)]
pub struct BookmarksWindow {
    pub open: WindowOpen,
    pub selected: Option<usize>,
    edit_name: bool,
}

impl BookmarksWindow {
    pub fn ui(ui: &mut Ui, app: &mut App) {
        let win = &mut app.ui.bookmarks_window;
        let mut action = Action::None;
        TableBuilder::new(ui)
            .columns(Size::remainder(), 3)
            .header(24.0, |mut row| {
                row.col(|ui| {
                    ui.label("Name");
                });
                row.col(|ui| {
                    ui.label("Offset");
                });
                row.col(|ui| {
                    ui.label("Region");
                });
            })
            .body(|body| {
                body.rows(20.0, app.meta.bookmarks.len(), |idx, mut row| {
                    row.col(|ui| {
                        if ui
                            .selectable_label(
                                win.selected == Some(idx),
                                &app.meta.bookmarks[idx].label,
                            )
                            .clicked()
                        {
                            win.selected = Some(idx);
                        }
                    });
                    row.col(|ui| {
                        if ui
                            .link(app.meta.bookmarks[idx].offset.to_string())
                            .clicked()
                        {
                            action = Action::Goto(app.meta.bookmarks[idx].offset);
                        }
                    });
                    row.col(|ui| {
                        let off = app.meta.bookmarks[idx].offset;
                        if let Some(region_key) =
                            find_most_specific_region_for_offset(&app.meta.regions, off)
                        {
                            let region = &app.meta.regions[region_key];
                            let ctx_menu = region_context_menu!(app, region, action);
                            if ui.link(&region.name).context_menu(ctx_menu).clicked() {
                                app.ui.regions_window.open = true;
                                app.ui.regions_window.selected_key = Some(region_key);
                            }
                        } else {
                            ui.label("<no region>");
                        }
                    });
                });
            });
        if let Some(idx) = win.selected {
            ui.separator();
            let mark = &mut app.meta.bookmarks[idx];
            ui.horizontal(|ui| {
                if win.edit_name {
                    if ui.text_edit_singleline(&mut mark.label).lost_focus() {
                        win.edit_name = false;
                    }
                } else {
                    ui.heading(&mark.label);
                }
                if ui.button("✏").clicked() {
                    win.edit_name ^= true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Offset");
                ui.add(egui::DragValue::new(&mut mark.offset));
            });
            ui.heading("Description");
            ui.text_edit_multiline(&mut mark.desc);
            if ui.button("Delete").clicked() {
                app.meta.bookmarks.remove(idx);
                win.selected = None;
            }
        }
        ui.separator();
        if ui.button("Add new at cursor").clicked() {
            app.meta.bookmarks.push(Bookmark {
                offset: app.edit_state.cursor,
                label: format!("New bookmark at {}", app.edit_state.cursor),
                desc: String::new(),
            })
        }
        match action {
            Action::None => {}
            Action::Goto(off) => {
                app.edit_state.cursor = off;
                app.center_view_on_offset(off);
                app.flash_cursor();
            }
        }
    }
}

enum Action {
    None,
    Goto(usize),
}
