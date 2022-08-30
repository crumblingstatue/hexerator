use egui_extras::Size;

use crate::app::FileDiffEntry;

use super::window_open::WindowOpen;

#[derive(Default)]
pub struct FileDiffResultWindow {
    pub diff_entries: Vec<FileDiffEntry>,
    pub open: WindowOpen,
}
impl FileDiffResultWindow {
    pub(crate) fn ui(ui: &mut egui_sfml::egui::Ui, app: &mut crate::app::App) {
        if app.ui.file_diff_result_window.diff_entries.is_empty() {
            ui.label("No difference");
            return;
        }
        let mut action = Action::None;
        egui_extras::TableBuilder::new(ui)
            .columns(Size::initial(100.0), 5)
            .resizable(true)
            .striped(true)
            .header(32.0, |mut row| {
                row.col(|ui| {
                    ui.label("My value");
                });
                row.col(|ui| {
                    ui.label("File value");
                });
                row.col(|ui| {
                    ui.label("Offset");
                });
                row.col(|ui| {
                    ui.label("Region");
                });
                row.col(|ui| {
                    ui.label("Bookmark");
                });
            })
            .body(|body| {
                body.rows(
                    20.0,
                    app.ui.file_diff_result_window.diff_entries.len(),
                    |idx, mut row| {
                        let entry = &app.ui.file_diff_result_window.diff_entries[idx];
                        row.col(|ui| {
                            ui.label(entry.my_val.to_string());
                        });
                        row.col(|ui| {
                            ui.label(entry.file_val.to_string());
                        });
                        row.col(|ui| {
                            if ui.link(entry.offset.to_string()).clicked() {
                                action = Action::GoToOffset(entry.offset);
                            }
                        });
                        row.col(|ui| {
                            match app.meta.find_most_specific_region_for_offset(entry.offset) {
                                Some(reg) => {
                                    let reg = &app.meta.regions[reg];
                                    ui.label(&reg.name);
                                }
                                None => {
                                    ui.label("[no region]");
                                }
                            }
                        });
                        row.col(|ui| {
                            match app.meta.bookmarks.iter().find(|b| b.offset == entry.offset) {
                                Some(bookmark) => {
                                    ui.label(&bookmark.label).on_hover_text(&bookmark.desc);
                                }
                                None => {
                                    ui.label("-");
                                }
                            }
                        });
                    },
                );
            });
        match action {
            Action::None => {}
            Action::GoToOffset(off) => {
                app.center_view_on_offset(off);
                app.edit_state.set_cursor(off);
                app.flash_cursor();
            }
        }
    }
}

enum Action {
    None,
    GoToOffset(usize),
}
