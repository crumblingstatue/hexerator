use {
    super::window_open::WindowOpen,
    crate::shell::{msg_fail, msg_if_fail},
    egui_extras::{Size, TableBuilder},
    egui_sfml::{egui, sfml::graphics::Font},
    sysinfo::{ProcessExt, Signal, SystemExt},
};

#[derive(Default)]
pub struct OpenProcessWindow {
    pub open: WindowOpen,
    pub sys: sysinfo::System,
    pub selected_pid: Option<sysinfo::Pid>,
    pub map_ranges: Vec<proc_maps::MapRange>,
    proc_name_filter_string: String,
    path_filter_string: String,
    addr_filter_string: String,
    pid_sort: Sort,
    addr_sort: Sort,
    size_sort: Sort,
    maps_sort_col: MapsSortColumn,
}

#[derive(Default, Clone, Copy)]
enum Sort {
    #[default]
    Ascending,
    Descending,
}

impl Sort {
    fn flip(&mut self) {
        *self = match *self {
            Sort::Ascending => Sort::Descending,
            Sort::Descending => Sort::Ascending,
        }
    }
}

fn sort_button(ui: &mut egui::Ui, label: &str, active: bool, sort: Sort) -> egui::Response {
    let arrow_str = if active {
        match sort {
            Sort::Ascending => "⏶",
            Sort::Descending => "⏷",
        }
    } else {
        "="
    };
    if active {
        ui.style_mut().visuals.faint_bg_color = egui::Color32::RED;
    }
    ui.button(format!("{} {}", label, arrow_str))
}

#[derive(Default, PartialEq, Eq)]
enum MapsSortColumn {
    #[default]
    StartOffset,
    Size,
}

impl OpenProcessWindow {
    pub(crate) fn ui(
        ui: &mut egui::Ui,
        gui: &mut crate::gui::Gui,
        app: &mut crate::app::App,
        font: &Font,
    ) {
        let win = &mut gui.open_process_window;
        if win.open.just_now() || ui.button("Refresh").clicked() {
            win.sys.refresh_processes();
        }
        if let &Some(pid) = &win.selected_pid {
            if ui.link("Back").clicked() {
                win.selected_pid = None;
            }
            if let Some(proc) = win.sys.process(pid) {
                ui.horizontal(|ui| {
                    if ui.button("Stop").clicked() {
                        proc.kill_with(Signal::Stop);
                    }
                    if ui.button("Continue").clicked() {
                        proc.kill_with(Signal::Continue);
                    }
                    if ui.button("Kill").clicked() {
                        proc.kill();
                    }
                });
            }
            TableBuilder::new(ui)
                .column(Size::initial(140.0))
                .column(Size::initial(80.0))
                .column(Size::initial(40.0))
                .column(Size::initial(300.0))
                .striped(true)
                .resizable(true)
                .header(20.0, |mut row| {
                    row.col(|ui| {
                        ui.horizontal(|ui| {
                            if sort_button(
                                ui,
                                "",
                                win.maps_sort_col == MapsSortColumn::StartOffset,
                                win.addr_sort,
                            )
                            .clicked()
                            {
                                win.maps_sort_col = MapsSortColumn::StartOffset;
                                win.addr_sort.flip();
                            }
                            ui.add(
                                egui::TextEdit::singleline(&mut win.addr_filter_string)
                                    .hint_text("🔎 Addr"),
                            );
                        });
                    });
                    row.col(|ui| {
                        if sort_button(
                            ui,
                            "size",
                            win.maps_sort_col == MapsSortColumn::Size,
                            win.size_sort,
                        )
                        .clicked()
                        {
                            win.maps_sort_col = MapsSortColumn::Size;
                            win.size_sort.flip();
                        }
                    });
                    row.col(|ui| {
                        ui.label("r/w/x");
                    });
                    row.col(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut win.path_filter_string)
                                .hint_text("🔎 Path"),
                        );
                    });
                })
                .body(|body| {
                    let mut filtered = win.map_ranges.clone();
                    filtered.retain(|range| {
                        if let Ok(addr) = usize::from_str_radix(&win.addr_filter_string, 16) {
                            if !(range.start() <= addr && range.start() + range.size() >= addr) {
                                return false;
                            }
                        }
                        if win.path_filter_string.is_empty() {
                            return true;
                        }
                        match range.filename() {
                            Some(path) => {
                                path.display().to_string().contains(&win.path_filter_string)
                            }
                            None => false,
                        }
                    });
                    filtered.sort_by(|range1, range2| match win.maps_sort_col {
                        MapsSortColumn::Size => match win.size_sort {
                            Sort::Ascending => range1.size().cmp(&range2.size()),
                            Sort::Descending => range1.size().cmp(&range2.size()).reverse(),
                        },
                        MapsSortColumn::StartOffset => match win.addr_sort {
                            Sort::Ascending => range1.start().cmp(&range2.start()),
                            Sort::Descending => range1.start().cmp(&range2.start()).reverse(),
                        },
                    });
                    body.rows(20.0, filtered.len(), |idx, mut row| {
                        let map_range = filtered[idx].clone();
                        row.col(|ui| {
                            if ui
                                .add_enabled(
                                    map_range.is_read(),
                                    egui::Button::new(format!("{:X}", map_range.start())),
                                )
                                .clicked()
                            {
                                msg_if_fail(
                                    app.load_proc_memory(
                                        pid,
                                        map_range.start(),
                                        map_range.size(),
                                        map_range.is_write(),
                                        font,
                                        &mut gui.msg_dialog,
                                    ),
                                    "Failed to load process memory",
                                    &mut gui.msg_dialog,
                                );
                                if let Ok(off) = usize::from_str_radix(&win.addr_filter_string, 16)
                                {
                                    let off = off - app.args.src.hard_seek.unwrap_or(0);
                                    app.edit_state.set_cursor(off);
                                    app.center_view_on_offset(off);
                                    app.hex_ui.flash_cursor();
                                }
                            }
                        });
                        row.col(|ui| {
                            ui.label(map_range.size().to_string());
                        });
                        row.col(|ui| {
                            ui.label(format!(
                                "{}{}{}",
                                if map_range.is_read() { "r" } else { "" },
                                if map_range.is_write() { "w" } else { "" },
                                if map_range.is_exec() { "x" } else { "" }
                            ));
                        });
                        row.col(|ui| {
                            ui.label(
                                map_range
                                    .filename()
                                    .map(|p| p.display().to_string())
                                    .unwrap_or_else(String::new),
                            );
                        });
                    });
                });
        } else {
            TableBuilder::new(ui)
                .column(Size::initial(100.0))
                .column(Size::remainder())
                .resizable(true)
                .striped(true)
                .header(20.0, |mut row| {
                    row.col(|ui| {
                        if sort_button(ui, "pid", true, win.pid_sort).clicked() {
                            win.pid_sort.flip()
                        }
                    });
                    row.col(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut win.proc_name_filter_string)
                                .hint_text("🔎 Name"),
                        );
                    });
                })
                .body(|body| {
                    let procs = win.sys.processes();
                    let mut pids: Vec<&sysinfo::Pid> = procs
                        .keys()
                        .filter(|&pid| procs[pid].name().contains(&win.proc_name_filter_string))
                        .collect();
                    pids.sort_by(|pid1, pid2| match win.pid_sort {
                        Sort::Ascending => pid1.cmp(pid2),
                        Sort::Descending => pid1.cmp(pid2).reverse(),
                    });
                    body.rows(20.0, pids.len(), |idx, mut row| {
                        let pid = pids[idx];
                        row.col(|ui| {
                            if ui
                                .selectable_label(Some(*pid) == win.selected_pid, pid.to_string())
                                .clicked()
                            {
                                win.selected_pid = Some(*pid);
                                match pid.to_string().parse() {
                                    Ok(pid) => match proc_maps::get_process_maps(pid) {
                                        Ok(ranges) => {
                                            win.map_ranges = ranges;
                                        }
                                        Err(e) => msg_fail(
                                            &e,
                                            "Failed to get map ranges for process",
                                            &mut gui.msg_dialog,
                                        ),
                                    },
                                    Err(e) => msg_fail(
                                        &e,
                                        "Failed to parse pid of process",
                                        &mut gui.msg_dialog,
                                    ),
                                }
                            }
                        });
                        row.col(|ui| {
                            ui.label(procs[pid].name());
                        });
                    });
                });
        }
        win.open.post_ui();
    }
}
