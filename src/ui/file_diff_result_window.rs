use std::path::PathBuf;

use egui_extras::Size;

use crate::{
    app::{read_source_to_buf, FileDiffEntry},
    meta::{find_most_specific_region_for_offset, Bookmark, RegionKey},
    region_context_menu,
    shell::msg_if_fail,
};

use super::window_open::WindowOpen;

#[derive(Default)]
pub struct FileDiffResultWindow {
    pub diff_entries: Vec<FileDiffEntry>,
    pub open: WindowOpen,
    pub path: PathBuf,
}
impl FileDiffResultWindow {
    pub(crate) fn ui(ui: &mut egui_sfml::egui::Ui, app: &mut crate::app::App) {
        if app.ui.file_diff_result_window.diff_entries.is_empty() {
            ui.label("No difference");
            return;
        }
        ui.label(app.ui.file_diff_result_window.path.display().to_string());
        ui.horizontal(|ui| {
            if ui
                .button("Filter unchanged")
                .on_hover_text("Keep only the unchanged values")
                .clicked()
            {
                let result: anyhow::Result<()> = try {
                    let file_data = std::fs::read(&app.ui.file_diff_result_window.path)?;
                    app.ui
                        .file_diff_result_window
                        .diff_entries
                        .retain(|en| en.file_val == file_data[en.offset]);
                };
                msg_if_fail(result, "Filter unchanged failed");
            }
            if ui
                .button("Filter changed")
                .on_hover_text("Keep only the values that changed")
                .clicked()
            {
                let result: anyhow::Result<()> = try {
                    let file_data =
                        read_source_to_buf(&app.ui.file_diff_result_window.path, &app.args.src)?;
                    app.ui
                        .file_diff_result_window
                        .diff_entries
                        .retain(|en| en.file_val != file_data[en.offset]);
                };
                msg_if_fail(result, "Filter unchanged failed");
            }
            if ui.button("Refresh").clicked() {
                let result: anyhow::Result<()> = try {
                    let file_data =
                        read_source_to_buf(&app.ui.file_diff_result_window.path, &app.args.src)?;
                    for en in &mut app.ui.file_diff_result_window.diff_entries {
                        en.file_val = file_data[en.offset];
                    }
                };
                msg_if_fail(result, "Refresh failed");
            }
        });
        ui.separator();
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
                            if ui
                                .link(entry.offset.to_string())
                                .context_menu(|ui| {
                                    if ui.button("Add bookmark").clicked() {
                                        let idx = app.meta.bookmarks.len();
                                        app.meta.bookmarks.push(Bookmark {
                                            offset: entry.offset,
                                            label: "New bookmark".into(),
                                            desc: String::new(),
                                        });
                                        app.ui.bookmarks_window.open.set(true);
                                        app.ui.bookmarks_window.selected = Some(idx);
                                    }
                                })
                                .clicked()
                            {
                                action = Action::Goto(entry.offset);
                            }
                        });
                        row.col(|ui| {
                            match find_most_specific_region_for_offset(
                                &app.meta.regions,
                                entry.offset,
                            ) {
                                Some(reg_key) => {
                                    let reg = &app.meta.regions[reg_key];
                                    ui.menu_button(&reg.name, |ui| {
                                        if ui.button("Remove region from results").clicked() {
                                            action = Action::RemoveRegion(reg_key);
                                            ui.close_menu();
                                        }
                                    })
                                    .response
                                    .context_menu(region_context_menu!(app, reg, action));
                                }
                                None => {
                                    ui.label("[no region]");
                                }
                            }
                        });
                        row.col(|ui| {
                            match app
                                .meta
                                .bookmarks
                                .iter()
                                .enumerate()
                                .find(|(_i, b)| b.offset == entry.offset)
                            {
                                Some((idx, bookmark)) => {
                                    if ui
                                        .link(&bookmark.label)
                                        .on_hover_text(&bookmark.desc)
                                        .clicked()
                                    {
                                        app.ui.bookmarks_window.open.set(true);
                                        app.ui.bookmarks_window.selected = Some(idx);
                                    }
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
            Action::Goto(off) => {
                app.center_view_on_offset(off);
                app.edit_state.set_cursor(off);
                app.flash_cursor();
            }
            Action::RemoveRegion(key) => app.ui.file_diff_result_window.diff_entries.retain(|en| {
                let reg = find_most_specific_region_for_offset(&app.meta.regions, en.offset);
                reg != Some(key)
            }),
        }
    }
}

enum Action {
    None,
    Goto(usize),
    RemoveRegion(RegionKey),
}
