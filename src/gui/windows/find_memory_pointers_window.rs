use {
    super::WindowCtxt,
    crate::{gui::window_open::WindowOpen, shell::msg_fail},
    egui_extras::{Column, TableBuilder},
};

#[derive(Default)]
pub struct FindMemoryPointersWindow {
    pub open: WindowOpen,
    pointers: Vec<PtrEntry>,
    filter_write: bool,
    filter_exec: bool,
}

#[derive(Clone, Copy)]
struct PtrEntry {
    src_idx: usize,
    ptr: usize,
    range_idx: usize,
    write: bool,
    execute: bool,
}

impl FindMemoryPointersWindow {
    pub fn ui(
        &mut self,
        WindowCtxt {
            ui, gui, app, font, ..
        }: WindowCtxt,
    ) {
        ui.style_mut().wrap = Some(false);
        let Some(pid) = gui.win.open_process.selected_pid else {
            ui.label("No selected pid.");
            return;
        };
        if self.open.just_now() {
            for (i, wnd) in app
                .data
                .array_windows::<{ (usize::BITS / 8) as usize }>()
                .enumerate()
            {
                let ptr = usize::from_le_bytes(*wnd);
                if let Some(pos) = gui.win.open_process.map_ranges.iter().position(|range| {
                    range.is_read() && range.start() <= ptr && range.start() + range.size() >= ptr
                }) {
                    let range = &gui.win.open_process.map_ranges[pos];
                    self.pointers.push(PtrEntry {
                        src_idx: i,
                        ptr,
                        range_idx: pos,
                        write: range.is_write(),
                        execute: range.is_exec(),
                    });
                }
            }
        }
        let mut action = Action::None;
        TableBuilder::new(ui)
            .column(Column::auto())
            .column(Column::auto())
            .column(Column::auto())
            .column(Column::remainder())
            .striped(true)
            .resizable(true)
            .header(20.0, |mut row| {
                row.col(|ui| {
                    ui.label("Location");
                });
                row.col(|ui| {
                    if ui.button("Region").clicked() {
                        self.pointers.sort_by_key(|p| {
                            gui.win.open_process.map_ranges[p.range_idx].filename()
                        });
                    }
                });
                row.col(|ui| {
                    ui.menu_button("w/x", |ui| {
                        ui.checkbox(&mut self.filter_write, "Write");
                        ui.checkbox(&mut self.filter_exec, "Execute");
                    });
                });
                row.col(|ui| {
                    if ui.button("Pointer").clicked() {
                        self.pointers.sort_by_key(|p| p.ptr);
                    }
                });
            })
            .body(|body| {
                let mut filtered = self.pointers.clone();
                filtered.retain(|ptr| {
                    if self.filter_exec && !ptr.execute {
                        return false;
                    }
                    if self.filter_write && !ptr.write {
                        return false;
                    }
                    true
                });
                body.rows(20.0, filtered.len(), |mut row| {
                    let en = &filtered[row.index()];
                    row.col(|ui| {
                        if ui.link(format!("{:X}", en.src_idx)).clicked() {
                            action = Action::Goto(en.src_idx);
                        }
                    });
                    row.col(|ui| {
                        let range = &gui.win.open_process.map_ranges[en.range_idx];
                        ui.label(
                            range
                                .filename()
                                .map(|p| p.display().to_string())
                                .unwrap_or_else(|| {
                                    format!("<anon> @ {:X} (size: {})", range.start(), range.size())
                                }),
                        );
                    });
                    row.col(|ui| {
                        let range = &gui.win.open_process.map_ranges[en.range_idx];
                        ui.label(format!(
                            "{}{}",
                            if range.is_write() { "w" } else { "" },
                            if range.is_exec() { "x" } else { "" }
                        ));
                    });
                    row.col(|ui| {
                        let range = &gui.win.open_process.map_ranges[en.range_idx];
                        if ui.link(format!("{:X}", en.ptr)).clicked() {
                            match app.load_proc_memory(
                                pid,
                                range.start(),
                                range.size(),
                                range.is_write(),
                                font,
                                &mut gui.msg_dialog,
                            ) {
                                Ok(()) => action = Action::Goto(en.ptr - range.start()),
                                Err(e) => {
                                    msg_fail(&e, "failed to load proc memory", &mut gui.msg_dialog)
                                }
                            }
                        }
                    });
                });
            });
        match action {
            Action::Goto(off) => {
                app.center_view_on_offset(off);
                app.edit_state.set_cursor(off);
                app.hex_ui.flash_cursor();
            }
            Action::None => {}
        }
        self.open.post_ui();
    }
}

enum Action {
    Goto(usize),
    None,
}
