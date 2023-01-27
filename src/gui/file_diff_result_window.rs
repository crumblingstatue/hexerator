use {
    crate::{
        app::{read_source_to_buf, FileDiffEntry},
        gui::window_open::WindowOpen,
        meta::{find_most_specific_region_for_offset, value_type::ValueType, Bookmark, RegionKey},
        region_context_menu,
        shell::msg_if_fail,
        Gui,
    },
    egui_extras::Column,
    std::{path::PathBuf, time::Instant},
};

pub struct FileDiffResultWindow {
    pub diff_entries: Vec<FileDiffEntry>,
    pub open: WindowOpen,
    pub path: PathBuf,
    pub auto_refresh: bool,
    pub auto_refresh_interval_ms: u32,
    pub last_refresh: Instant,
}

impl Default for FileDiffResultWindow {
    fn default() -> Self {
        Self {
            diff_entries: Default::default(),
            open: Default::default(),
            path: Default::default(),
            auto_refresh: Default::default(),
            auto_refresh_interval_ms: Default::default(),
            last_refresh: Instant::now(),
        }
    }
}
impl FileDiffResultWindow {
    pub(crate) fn ui(ui: &mut egui::Ui, gui: &mut Gui, app: &mut crate::app::App) {
        if gui.file_diff_result_window.diff_entries.is_empty() {
            ui.label("No difference");
            return;
        }
        ui.label(gui.file_diff_result_window.path.display().to_string());
        ui.horizontal(|ui| {
            if ui
                .button("Filter unchanged")
                .on_hover_text("Keep only the unchanged values")
                .clicked()
            {
                let result: anyhow::Result<()> = try {
                    let file_data =
                        read_source_to_buf(&gui.file_diff_result_window.path, &app.args.src)?;
                    gui.file_diff_result_window
                        .diff_entries
                        .retain(|en| en.file_val == file_data[en.offset]);
                };
                msg_if_fail(result, "Filter unchanged failed", &mut gui.msg_dialog);
            }
            if ui
                .button("Filter changed")
                .on_hover_text("Keep only the values that changed")
                .clicked()
            {
                let result: anyhow::Result<()> = try {
                    let file_data =
                        read_source_to_buf(&gui.file_diff_result_window.path, &app.args.src)?;
                    gui.file_diff_result_window
                        .diff_entries
                        .retain(|en| en.file_val != file_data[en.offset]);
                };
                msg_if_fail(result, "Filter unchanged failed", &mut gui.msg_dialog);
            }
        });
        ui.horizontal(|ui| {
            if ui.button("Refresh").clicked()
                || (gui.file_diff_result_window.auto_refresh
                    && gui
                        .file_diff_result_window
                        .last_refresh
                        .elapsed()
                        .as_millis()
                        >= u128::from(gui.file_diff_result_window.auto_refresh_interval_ms))
            {
                gui.file_diff_result_window.last_refresh = Instant::now();
                let result: anyhow::Result<()> = try {
                    let file_data =
                        read_source_to_buf(&gui.file_diff_result_window.path, &app.args.src)?;
                    for en in &mut gui.file_diff_result_window.diff_entries {
                        en.file_val = file_data[en.offset];
                    }
                };
                msg_if_fail(result, "Refresh failed", &mut gui.msg_dialog);
            }
            ui.checkbox(
                &mut gui.file_diff_result_window.auto_refresh,
                "Auto refresh",
            );
            ui.label("Interval");
            ui.add(egui::DragValue::new(
                &mut gui.file_diff_result_window.auto_refresh_interval_ms,
            ));
        });
        ui.separator();
        let mut action = Action::None;
        egui_extras::TableBuilder::new(ui)
            .columns(Column::auto(), 4)
            .column(Column::remainder())
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
                    gui.file_diff_result_window.diff_entries.len(),
                    |idx, mut row| {
                        let entry = &gui.file_diff_result_window.diff_entries[idx];
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
                                        let idx = app.meta_state.meta.bookmarks.len();
                                        app.meta_state.meta.bookmarks.push(Bookmark {
                                            offset: entry.offset,
                                            label: "New bookmark".into(),
                                            desc: String::new(),
                                            value_type: ValueType::None,
                                        });
                                        gui.bookmarks_window.open.set(true);
                                        gui.bookmarks_window.selected = Some(idx);
                                    }
                                })
                                .clicked()
                            {
                                action = Action::Goto(entry.offset);
                            }
                        });
                        row.col(|ui| {
                            match find_most_specific_region_for_offset(
                                &app.meta_state.meta.low.regions,
                                entry.offset,
                            ) {
                                Some(reg_key) => {
                                    let reg = &app.meta_state.meta.low.regions[reg_key];
                                    ui.menu_button(&reg.name, |ui| {
                                        if ui.button("Remove region from results").clicked() {
                                            action = Action::RemoveRegion(reg_key);
                                            ui.close_menu();
                                        }
                                    })
                                    .response
                                    .context_menu(|ui| region_context_menu!(ui, app, reg, action));
                                }
                                None => {
                                    ui.label("[no region]");
                                }
                            }
                        });
                        row.col(|ui| {
                            match app
                                .meta_state
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
                                        gui.bookmarks_window.open.set(true);
                                        gui.bookmarks_window.selected = Some(idx);
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
                app.hex_ui.flash_cursor();
            }
            Action::RemoveRegion(key) => gui.file_diff_result_window.diff_entries.retain(|en| {
                let reg = find_most_specific_region_for_offset(
                    &app.meta_state.meta.low.regions,
                    en.offset,
                );
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
