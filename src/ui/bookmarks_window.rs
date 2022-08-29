use egui_sfml::egui::{self, Ui};

use crate::app::{App, Bookmark};

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
        let bookmarks = &mut app.bookmarks;
        let mut jump_to = None;
        for (i, mark) in bookmarks.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                if ui
                    .selectable_label(win.selected == Some(i), &mark.label)
                    .clicked()
                {
                    win.selected = Some(i);
                }
                if ui.button("⮩").clicked() {
                    jump_to = Some(mark.offset);
                }
            });
        }
        if let Some(idx) = win.selected {
            ui.separator();
            let mark = &mut bookmarks[idx];
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
                bookmarks.remove(idx);
                win.selected = None;
            }
        }
        ui.separator();
        if ui.button("Add new at cursor").clicked() {
            app.bookmarks.push(Bookmark {
                offset: app.edit_state.cursor,
                label: format!("New bookmark at {}", app.edit_state.cursor),
                desc: String::new(),
            })
        }
        if let Some(off) = jump_to {
            app.edit_state.cursor = off;
            app.center_view_on_offset(off);
            app.flash_cursor();
        }
    }
}
