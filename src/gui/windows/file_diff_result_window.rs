use {
    super::{WinCtx, WindowOpen},
    crate::{
        app::read_source_to_buf,
        gui::windows::regions_window::region_context_menu,
        meta::{
            find_most_specific_region_for_offset, value_type::ValueType, Bookmark, Meta, RegionKey,
        },
        shell::msg_if_fail,
    },
    egui_extras::Column,
    std::{path::PathBuf, time::Instant},
};

pub struct FileDiffResultWindow {
    pub file_data: Vec<u8>,
    pub offsets: Vec<usize>,
    pub open: WindowOpen,
    pub path: PathBuf,
    pub auto_refresh: bool,
    pub auto_refresh_interval_ms: u32,
    pub last_refresh: Instant,
}

impl Default for FileDiffResultWindow {
    fn default() -> Self {
        Self {
            offsets: Default::default(),
            open: Default::default(),
            path: Default::default(),
            auto_refresh: Default::default(),
            auto_refresh_interval_ms: Default::default(),
            last_refresh: Instant::now(),
            file_data: Vec::new(),
        }
    }
}
impl super::Window for FileDiffResultWindow {
    fn ui(
        &mut self,
        WinCtx {
            ui, gui, app, font, ..
        }: WinCtx,
    ) {
        if self.offsets.is_empty() {
            ui.label("No difference");
            return;
        }
        ui.label(self.path.display().to_string());
        ui.horizontal(|ui| {
            if ui
                .button("🗁 Open this")
                .on_hover_text("Open the diffed-against file Hexerator")
                .clicked()
            {
                let prev_pref = app.preferences.keep_meta;
                let prev_path = app.src_args.file.clone();
                app.preferences.keep_meta = true;
                let result = app.load_file(self.path.clone(), false, font, &mut gui.msg_dialog);
                app.preferences.keep_meta = prev_pref;
                if msg_if_fail(result, "Failed to load file", &mut gui.msg_dialog).is_none() {
                    if let Some(path) = prev_path {
                        msg_if_fail(
                            app.diff_with_file(path, self),
                            "Failed to diff",
                            &mut gui.msg_dialog,
                        );
                    }
                }
            }
            if ui.button("🖹 Diff with...").on_hover_text("Diff with another file").clicked() {
                gui.fileops.diff_with_file(app.source_file());
            }
        });
        ui.separator();
        ui.horizontal(|ui| {
            if ui
                .button("Filter unchanged")
                .on_hover_text("Keep only the unchanged values")
                .clicked()
            {
                let result: anyhow::Result<()> = try {
                    let file_data = read_source_to_buf(&self.path, &app.src_args)?;
                    self.offsets.retain(|&offs| self.file_data[offs] == file_data[offs]);
                };
                msg_if_fail(result, "Filter unchanged failed", &mut gui.msg_dialog);
            }
            if ui
                .button("Filter changed")
                .on_hover_text("Keep only the values that changed")
                .clicked()
            {
                let result: anyhow::Result<()> = try {
                    let file_data = read_source_to_buf(&self.path, &app.src_args)?;
                    self.offsets.retain(|&offs| self.file_data[offs] != file_data[offs]);
                };
                msg_if_fail(result, "Filter unchanged failed", &mut gui.msg_dialog);
            }
            if ui.button("Highlight all").clicked() {
                gui.highlight_set.clear();
                for &offs in &self.offsets {
                    gui.highlight_set.insert(offs);
                    if let Some((_, bm)) =
                        Meta::bookmark_for_offset(&app.meta_state.meta.bookmarks, offs)
                    {
                        for i in 1..bm.value_type.byte_len() {
                            gui.highlight_set.insert(offs + i);
                        }
                    }
                }
            }
        });
        ui.horizontal(|ui| {
            if ui.button("Refresh").clicked()
                || (self.auto_refresh
                    && self.last_refresh.elapsed().as_millis()
                        >= u128::from(self.auto_refresh_interval_ms))
            {
                self.last_refresh = Instant::now();
                let result: anyhow::Result<()> = try {
                    self.file_data = read_source_to_buf(&self.path, &app.src_args)?;
                };
                msg_if_fail(result, "Refresh failed", &mut gui.msg_dialog);
            }
            ui.checkbox(&mut self.auto_refresh, "Auto refresh");
            ui.label("Interval");
            ui.add(egui::DragValue::new(&mut self.auto_refresh_interval_ms));
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
                body.rows(20.0, self.offsets.len(), |mut row| {
                    let offs = self.offsets[row.index()];
                    let bm = Meta::bookmark_for_offset(&app.meta_state.meta.bookmarks, offs)
                        .map(|(_, bm)| bm);
                    row.col(|ui| {
                        let s = match bm {
                            Some(bm) => bm
                                .value_type
                                .read(&app.data[offs..])
                                .map(|v| v.to_string())
                                .unwrap_or("err".into()),
                            None => app.data[offs].to_string(),
                        };
                        ui.label(s);
                    });
                    row.col(|ui| {
                        let s = match bm {
                            Some(bm) => bm
                                .value_type
                                .read(&self.file_data[offs..])
                                .map(|v| v.to_string())
                                .unwrap_or("err".into()),
                            None => self.file_data[offs].to_string(),
                        };
                        ui.label(s);
                    });
                    row.col(|ui| {
                        let re = ui.link(offs.to_string());
                        re.context_menu(|ui| {
                            if ui.button("Add bookmark").clicked() {
                                let idx = app.meta_state.meta.bookmarks.len();
                                app.meta_state.meta.bookmarks.push(Bookmark {
                                    offset: offs,
                                    label: "New bookmark".into(),
                                    desc: String::new(),
                                    value_type: ValueType::None,
                                });
                                gui.win.bookmarks.open.set(true);
                                gui.win.bookmarks.selected = Some(idx);
                                ui.close_menu();
                            }
                        });
                        if re.clicked() {
                            action = Action::Goto(offs);
                        }
                    });
                    row.col(|ui| {
                        match find_most_specific_region_for_offset(
                            &app.meta_state.meta.low.regions,
                            offs,
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
                                .context_menu(|ui| {
                                    region_context_menu(
                                        ui,
                                        reg,
                                        reg_key,
                                        &app.meta_state.meta,
                                        &mut app.cmd,
                                        &mut gui.cmd,
                                    )
                                });
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
                            .find(|(_i, b)| b.offset == offs)
                        {
                            Some((idx, bookmark)) => {
                                if ui.link(&bookmark.label).on_hover_text(&bookmark.desc).clicked()
                                {
                                    gui.win.bookmarks.open.set(true);
                                    gui.win.bookmarks.selected = Some(idx);
                                }
                            }
                            None => {
                                ui.label("-");
                            }
                        }
                    });
                });
            });
        match action {
            Action::None => {}
            Action::Goto(off) => {
                app.center_view_on_offset(off);
                app.edit_state.set_cursor(off);
                app.hex_ui.flash_cursor();
            }
            Action::RemoveRegion(key) => self.offsets.retain(|&offs| {
                let reg =
                    find_most_specific_region_for_offset(&app.meta_state.meta.low.regions, offs);
                reg != Some(key)
            }),
        }
    }

    fn title(&self) -> &str {
        "File Diff results"
    }
}

enum Action {
    None,
    Goto(usize),
    RemoveRegion(RegionKey),
}
