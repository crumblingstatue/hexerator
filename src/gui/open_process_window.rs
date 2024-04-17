use {
    super::{message_dialog::MessageDialog, window_open::WindowOpen},
    crate::shell::{msg_fail, msg_if_fail},
    egui_extras::{Column, TableBuilder},
    egui_sfml::sfml::graphics::Font,
    std::process::Command,
    sysinfo::Signal,
};

type MapRanges = Vec<proc_maps::MapRange>;

#[derive(Default)]
pub struct OpenProcessWindow {
    pub open: WindowOpen,
    pub sys: sysinfo::System,
    pub selected_pid: Option<sysinfo::Pid>,
    pub map_ranges: MapRanges,
    proc_name_filter_string: String,
    path_filter_string: String,
    addr_filter_string: String,
    pid_sort: Sort,
    addr_sort: Sort,
    size_sort: Sort,
    maps_sort_col: MapsSortColumn,
    perm_filters: PermFilters,
    modal: Option<Modal>,
}

#[derive(Default)]
pub struct PermFilters {
    read: bool,
    write: bool,
    execute: bool,
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
    ui.button(format!("{label} {arrow_str}"))
}

#[derive(Default, PartialEq, Eq)]
enum MapsSortColumn {
    #[default]
    StartOffset,
    Size,
}

enum Modal {
    RunCommand(RunCommand),
}

impl Modal {
    fn run_command() -> Self {
        Self::RunCommand(RunCommand {
            command: String::new(),
            just_opened: true,
        })
    }
}

struct RunCommand {
    command: String,
    just_opened: bool,
}

impl OpenProcessWindow {
    pub(crate) fn ui(
        ui: &mut egui::Ui,
        gui: &mut crate::gui::Gui,
        app: &mut crate::app::App,
        font: &Font,
    ) {
        let win = &mut gui.open_process_window;
        if let Some(modal) = &mut win.modal {
            let mut close_modal = false;
            ui.horizontal(|ui| match modal {
                Modal::RunCommand(run_command) => {
                    ui.label("Command");
                    let re = ui.text_edit_singleline(&mut run_command.command);
                    if run_command.just_opened {
                        re.request_focus();
                        run_command.just_opened = false;
                    }
                    let enter = ui.input(|inp| inp.key_pressed(egui::Key::Enter));
                    if ui.button("Run").clicked() || (re.lost_focus() && enter) {
                        match Command::new(&mut run_command.command).spawn() {
                            Ok(child) => {
                                let pid = child.id();
                                win.selected_pid = Some(sysinfo::Pid::from_u32(pid));
                                refresh_proc_maps(pid, &mut win.map_ranges, &mut gui.msg_dialog);
                                // Make sure this process is visible for sysinfo to kill/stop/etc.
                                win.sys.refresh_processes();
                                close_modal = true;
                            }
                            Err(e) => msg_fail(&e, "Run command error", &mut gui.msg_dialog),
                        }
                    }
                    if ui.button("Cancel").clicked() {
                        close_modal = true;
                    }
                }
            });
            if close_modal {
                win.modal = None;
            }
            ui.set_enabled(false);
        }
        ui.horizontal(|ui| {
            match win.selected_pid {
                None => {
                    if win.open.just_now() || ui.button("Refresh processes").clicked() {
                        win.sys.refresh_processes();
                    }
                }
                Some(pid) => {
                    if ui.button("Refresh memory maps").clicked() {
                        refresh_proc_maps(pid.as_u32(), &mut win.map_ranges, &mut gui.msg_dialog);
                    }
                }
            }
            if ui.button("Run command...").clicked() {
                win.modal = Some(Modal::run_command());
            }
        });
        if let &Some(pid) = &win.selected_pid {
            ui.heading(format!("Virtual memory maps for pid {pid}"));
            if ui.link("Back to process list").clicked() {
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
            let mut filtered = win.map_ranges.clone();
            TableBuilder::new(ui)
                .max_scroll_height(400.0)
                .column(Column::auto())
                .column(Column::auto())
                .column(Column::auto())
                .column(Column::remainder())
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
                        ui.add(egui::Label::new("r/w/x").sense(egui::Sense::click()))
                            .context_menu(|ui| {
                                ui.label("Filter");
                                ui.separator();
                                ui.checkbox(&mut win.perm_filters.read, "Read");
                                ui.checkbox(&mut win.perm_filters.write, "Write");
                                ui.checkbox(&mut win.perm_filters.execute, "Execute");
                            });
                    });
                    row.col(|ui| {
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut win.path_filter_string)
                                    .hint_text("🔎 Path"),
                            );
                            if ui
                                .button("🗑")
                                .on_hover_text("Remove filtered paths")
                                .clicked()
                            {
                                win.map_ranges.retain(|range| {
                                    let mut retain = true;
                                    if let Some(filename) = range.filename() {
                                        if filename
                                            .display()
                                            .to_string()
                                            .contains(&win.path_filter_string)
                                        {
                                            retain = false;
                                        }
                                    }
                                    retain
                                });
                                win.path_filter_string.clear();
                            }
                        });
                    });
                })
                .body(|body| {
                    filtered.retain(|range| {
                        if win.perm_filters.read && !range.is_read() {
                            return false;
                        }
                        if win.perm_filters.write && !range.is_write() {
                            return false;
                        }
                        if win.perm_filters.execute && !range.is_exec() {
                            return false;
                        }
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
                    body.rows(20.0, filtered.len(), |mut row| {
                        let map_range = filtered[row.index()].clone();
                        // This range is likely open in the editor (range contains hard_seek)
                        let mut likely_open = false;
                        if let Some(hard_seek) = app.src_args.hard_seek {
                            if hard_seek >= map_range.start()
                                && hard_seek < map_range.start() + map_range.size()
                            {
                                likely_open = true;
                            }
                        }
                        row.col(|ui| {
                            let txt = format!("{:X}", map_range.start());
                            let mut rich_txt = egui::RichText::new(&txt);
                            if likely_open {
                                rich_txt = rich_txt.color(egui::Color32::YELLOW);
                            }
                            let mut is_button = false;
                            let re = if map_range.is_read() {
                                is_button = true;
                                ui.add(egui::Button::new(rich_txt))
                            } else {
                                ui.add(egui::Label::new(rich_txt).sense(egui::Sense::click()))
                            };
                            re.context_menu(|ui| {
                                if ui.button("📋 Copy to clipboard").clicked() {
                                    crate::app::set_clipboard_string(
                                        &mut app.clipboard,
                                        &mut gui.msg_dialog,
                                        &txt,
                                    );
                                    ui.close_menu();
                                }
                            });
                            if re.clicked() && is_button {
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
                                    let off = off - app.src_args.hard_seek.unwrap_or(0);
                                    app.edit_state.set_cursor(off);
                                    app.center_view_on_offset(off);
                                    app.hex_ui.flash_cursor();
                                }
                            }
                        });
                        row.col(|ui| {
                            let txt = map_range.size().to_string();
                            ui.add(egui::Label::new(&txt).sense(egui::Sense::click()))
                                .context_menu(|ui| {
                                    if ui.button("📋 Copy to clipboard").clicked() {
                                        crate::app::set_clipboard_string(
                                            &mut app.clipboard,
                                            &mut gui.msg_dialog,
                                            &txt,
                                        );
                                        ui.close_menu();
                                    }
                                });
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
                            let txt = map_range
                                .filename()
                                .map(|p| p.display().to_string())
                                .unwrap_or_default();
                            ui.add(egui::Label::new(&txt).sense(egui::Sense::click()))
                                .context_menu(|ui| {
                                    if ui.button("📋 Copy to clipboard").clicked() {
                                        crate::app::set_clipboard_string(
                                            &mut app.clipboard,
                                            &mut gui.msg_dialog,
                                            &txt,
                                        );
                                        ui.close_menu();
                                    }
                                });
                        });
                    });
                });
            ui.separator();
            #[expect(
                clippy::cast_precision_loss,
                reason = "This is just an approximation of data size"
            )]
            ui.label(format!(
                "{}/{} maps shown ({})",
                filtered.len(),
                win.map_ranges.len(),
                human_bytes::human_bytes(
                    filtered.iter().map(|range| range.size()).sum::<usize>() as f64
                )
            ));
        } else {
            TableBuilder::new(ui)
                .column(Column::auto())
                .column(Column::remainder())
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
                    let filt_str = win.proc_name_filter_string.to_ascii_lowercase();
                    let mut pids: Vec<&sysinfo::Pid> = procs
                        .keys()
                        .filter(|&pid| procs[pid].name().to_ascii_lowercase().contains(&filt_str))
                        .collect();
                    pids.sort_by(|pid1, pid2| match win.pid_sort {
                        Sort::Ascending => pid1.cmp(pid2),
                        Sort::Descending => pid1.cmp(pid2).reverse(),
                    });
                    body.rows(20.0, pids.len(), |mut row| {
                        let pid = pids[row.index()];
                        row.col(|ui| {
                            if ui
                                .selectable_label(Some(*pid) == win.selected_pid, pid.to_string())
                                .clicked()
                            {
                                win.selected_pid = Some(*pid);
                                match pid.to_string().parse() {
                                    Ok(pid) => refresh_proc_maps(
                                        pid,
                                        &mut win.map_ranges,
                                        &mut gui.msg_dialog,
                                    ),
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

fn refresh_proc_maps(pid: u32, win_map_ranges: &mut MapRanges, msg: &mut MessageDialog) {
    #[expect(
        clippy::cast_possible_wrap,
        reason = "Hopefully pid isn't greater than 2^31"
    )]
    match proc_maps::get_process_maps(pid as _) {
        Ok(ranges) => {
            *win_map_ranges = ranges;
        }
        Err(e) => msg_fail(&e, "Failed to get map ranges for process", msg),
    }
}
