use {
    super::{WinCtx, WindowOpen},
    crate::{
        gui::message_dialog::MessageDialog,
        shell::{msg_fail, msg_if_fail},
        util::human_size,
    },
    egui_extras::{Column, TableBuilder},
    egui_file_dialog::FileDialog,
    proc_maps::MapRange,
    smart_default::SmartDefault,
    std::{path::PathBuf, process::Command},
    sysinfo::{ProcessesToUpdate, Signal},
};

type MapRanges = Vec<proc_maps::MapRange>;

#[derive(SmartDefault)]
pub struct OpenProcessWindow {
    pub open: WindowOpen,
    pub sys: sysinfo::System,
    pub selected_pid: Option<sysinfo::Pid>,
    pub map_ranges: MapRanges,
    pid_sort: Sort,
    addr_sort: Sort,
    size_sort: Sort,
    maps_sort_col: MapsSortColumn,
    pub filters: Filters,
    modal: Option<Modal>,
    find: FindState,
    pub default_meta_path: Option<PathBuf>,
    #[default = true]
    use_default_meta_path: bool,
}

#[derive(Default)]
pub struct Filters {
    pub path: String,
    pub addr: String,
    pub proc_name: String,
    pub perms: PermFilters,
}

#[derive(Default)]
struct FindState {
    open: bool,
    input: String,
    results: Vec<MapFindResults>,
}

struct MapFindResults {
    map: MapRange,
    offsets: Vec<usize>,
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
            file_dialog: FileDialog::new(),
        })
    }
}

struct RunCommand {
    command: String,
    just_opened: bool,
    file_dialog: FileDialog,
}

impl super::Window for OpenProcessWindow {
    fn ui(
        &mut self,
        WinCtx {
            ui,
            gui,
            app,
            font_size,
            line_spacing,
            ..
        }: WinCtx,
    ) {
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
        if let Some(modal) = &mut self.modal {
            let mut close_modal = false;
            ui.horizontal(|ui| match modal {
                Modal::RunCommand(run_command) => {
                    run_command.file_dialog.update(ui.ctx());
                    ui.label("Command");
                    if let Some(file_path) = run_command.file_dialog.take_selected() {
                        run_command.command.push_str(&format!("\"{}\"", file_path.display()));
                    }
                    let re = ui.text_edit_singleline(&mut run_command.command);
                    if run_command.just_opened {
                        re.request_focus();
                        run_command.just_opened = false;
                    }
                    let enter = ui.input(|inp| inp.key_pressed(egui::Key::Enter));
                    match shlex::split(&run_command.command) {
                        Some(tokens) => {
                            let mut tokens = tokens.into_iter();
                            if ui.button("Run").clicked() || (re.lost_focus() && enter) {
                                if let Some(first) = tokens.next() {
                                    match Command::new(first).args(tokens).spawn() {
                                        Ok(child) => {
                                            let pid = child.id();
                                            self.selected_pid = Some(sysinfo::Pid::from_u32(pid));
                                            refresh_proc_maps(
                                                pid,
                                                &mut self.map_ranges,
                                                &mut gui.msg_dialog,
                                            );
                                            // Make sure this process is visible for sysinfo to kill/stop/etc.
                                            self.sys
                                                .refresh_processes(ProcessesToUpdate::All, true);
                                            close_modal = true;
                                        }
                                        Err(e) => {
                                            msg_fail(&e, "Run command error", &mut gui.msg_dialog)
                                        }
                                    }
                                }
                            }
                        }
                        None => {
                            ui.add_enabled(false, egui::Button::new("Run"));
                        }
                    }
                    if ui.button("Add file...").clicked() {
                        run_command.file_dialog.select_file();
                    }
                    if ui.button("Cancel").clicked() {
                        close_modal = true;
                    }
                }
            });
            if close_modal {
                self.modal = None;
            }
            ui.disable();
        }
        ui.horizontal(|ui| {
            match self.selected_pid {
                None => {
                    if self.open.just_now() || ui.button("Refresh processes").clicked() {
                        self.sys.refresh_processes(ProcessesToUpdate::All, true);
                    }
                }
                Some(pid) => {
                    if ui.button("Refresh memory maps").clicked() {
                        refresh_proc_maps(pid.as_u32(), &mut self.map_ranges, &mut gui.msg_dialog);
                    }
                    if ui
                        .selectable_label(self.find.open, "🔍 Find...")
                        .on_hover_text("Find values across all map ranges")
                        .clicked()
                    {
                        self.find.open ^= true;
                    }
                }
            }
            if ui.button("Run command...").clicked() {
                self.modal = Some(Modal::run_command());
            }
            if let Some(path) = &self.default_meta_path {
                ui.checkbox(
                    &mut self.use_default_meta_path,
                    format!("Use metafile {}", path.display()),
                );
            }
        });
        if let &Some(pid) = &self.selected_pid {
            if self.find.open {
                ui.text_edit_singleline(&mut self.find.input);
                match self.find.input.parse::<u8>() {
                    Ok(num) => {
                        if ui.button("Find").clicked() {
                            self.find.results.clear();
                            for range in self
                                .map_ranges
                                .iter()
                                .filter(|range| should_retain_range(&self.filters, range))
                            {
                                match app.load_proc_memory(
                                    pid,
                                    range.start(),
                                    range.size(),
                                    range.is_write(),
                                    &mut gui.msg_dialog,
                                    font_size,
                                    line_spacing,
                                ) {
                                    Ok(()) => {
                                        let mut offsets = Vec::new();
                                        for offset in memchr::memchr_iter(num, &app.data) {
                                            offsets.push(offset);
                                        }
                                        self.find.results.push(MapFindResults {
                                            map: range.clone(),
                                            offsets,
                                        });
                                    }
                                    Err(e) => msg_fail(&e, "Error", &mut gui.msg_dialog),
                                }
                            }
                        }
                        if !self.find.results.is_empty() && ui.button("Retain").clicked() {
                            self.find.results.retain_mut(|result| {
                                match app.load_proc_memory(
                                    pid,
                                    result.map.start(),
                                    result.map.size(),
                                    result.map.is_write(),
                                    &mut gui.msg_dialog,
                                    font_size,
                                    line_spacing,
                                ) {
                                    Ok(()) => {
                                        result.offsets.retain(|offset| {
                                            app.data.get(*offset).is_some_and(|byte| *byte == num)
                                        });
                                        !result.offsets.is_empty()
                                    }
                                    Err(e) => {
                                        msg_fail(&e, "Error", &mut gui.msg_dialog);
                                        false
                                    }
                                }
                            });
                        }
                    }
                    Err(e) => {
                        ui.add_enabled(false, egui::Button::new("Find"))
                            .on_disabled_hover_text(format!("{e}"));
                    }
                }

                let result_count: usize =
                    self.find.results.iter().map(|res| res.offsets.len()).sum();

                if result_count < 30 {
                    for (i, result) in self.find.results.iter().enumerate() {
                        let label = format!(
                            "{}..={} ({}) @ {:?}",
                            result.map.start(),
                            result.map.start() + result.map.size(),
                            result.map.size(),
                            result.map.filename(),
                        );
                        let map_open = app
                            .src_args
                            .hard_seek
                            .is_some_and(|offset| offset == result.map.start());
                        let _ = ui.selectable_label(map_open, label);
                        ui.indent(egui::Id::new("result_ident").with(i), |ui| {
                            for offset in &result.offsets {
                                ui.horizontal(|ui| {
                                    if ui.button(format!("{offset:X}")).clicked() {
                                        if !map_open {
                                            match app.load_proc_memory(
                                                pid,
                                                result.map.start(),
                                                result.map.size(),
                                                result.map.is_write(),
                                                &mut gui.msg_dialog,
                                                font_size,
                                                line_spacing,
                                            ) {
                                                Ok(()) => {
                                                    app.search_focus(*offset);
                                                }
                                                Err(e) => {
                                                    msg_fail(&e, "Error", &mut gui.msg_dialog)
                                                }
                                            }
                                            if let Some(path) = &self.default_meta_path
                                                && self.use_default_meta_path
                                            {
                                                let result =
                                                    app.consume_meta_from_file(path.clone());
                                                msg_if_fail(
                                                    result,
                                                    "Failed to consume metafile",
                                                    &mut gui.msg_dialog,
                                                );
                                            }
                                        } else {
                                            app.search_focus(*offset);
                                        }
                                    }
                                    if map_open {
                                        let mut s = String::new();
                                        ui.label(
                                            app.data
                                                .get(*offset)
                                                .map(|off| {
                                                    s = off.to_string();
                                                    s.as_str()
                                                })
                                                .unwrap_or("??"),
                                        );
                                    }
                                });
                            }
                        });
                    }
                }

                ui.label(format!("{} Results", result_count));
                return;
            }
            ui.heading(format!("Virtual memory maps for pid {pid}"));
            if ui.link("Back to process list").clicked() {
                self.sys.refresh_processes(ProcessesToUpdate::All, true);
                self.selected_pid = None;
            }
            if let Some(proc) = self.sys.process(pid) {
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
            let mut filtered = self.map_ranges.clone();
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
                                self.maps_sort_col == MapsSortColumn::StartOffset,
                                self.addr_sort,
                            )
                            .clicked()
                            {
                                self.maps_sort_col = MapsSortColumn::StartOffset;
                                self.addr_sort.flip();
                            }
                            ui.add(
                                egui::TextEdit::singleline(&mut self.filters.addr)
                                    .hint_text("🔎 Addr"),
                            );
                        });
                    });
                    row.col(|ui| {
                        if sort_button(
                            ui,
                            "size",
                            self.maps_sort_col == MapsSortColumn::Size,
                            self.size_sort,
                        )
                        .clicked()
                        {
                            self.maps_sort_col = MapsSortColumn::Size;
                            self.size_sort.flip();
                        }
                    });
                    row.col(|ui| {
                        ui.add(egui::Label::new("r/w/x").sense(egui::Sense::click())).context_menu(
                            |ui| {
                                ui.label("Filter");
                                ui.separator();
                                ui.checkbox(&mut self.filters.perms.read, "Read");
                                ui.checkbox(&mut self.filters.perms.write, "Write");
                                ui.checkbox(&mut self.filters.perms.execute, "Execute");
                            },
                        );
                    });
                    row.col(|ui| {
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut self.filters.path)
                                    .hint_text("🔎 Path"),
                            );
                            if ui.button("🗑").on_hover_text("Remove filtered paths").clicked() {
                                self.map_ranges.retain(|range| {
                                    let mut retain = true;
                                    if let Some(filename) = range.filename() {
                                        if filename
                                            .display()
                                            .to_string()
                                            .contains(&self.filters.path)
                                        {
                                            retain = false;
                                        }
                                    }
                                    retain
                                });
                                self.filters.path.clear();
                            }
                        });
                    });
                })
                .body(|body| {
                    filtered.retain(|range| should_retain_range(&self.filters, range));
                    filtered.sort_by(|range1, range2| match self.maps_sort_col {
                        MapsSortColumn::Size => match self.size_sort {
                            Sort::Ascending => range1.size().cmp(&range2.size()),
                            Sort::Descending => range1.size().cmp(&range2.size()).reverse(),
                        },
                        MapsSortColumn::StartOffset => match self.addr_sort {
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
                                        &mut gui.msg_dialog,
                                        font_size,
                                        line_spacing,
                                    ),
                                    "Failed to load process memory",
                                    &mut gui.msg_dialog,
                                );
                                if let Some(path) = &self.default_meta_path
                                    && self.use_default_meta_path
                                {
                                    let result = app.consume_meta_from_file(path.clone());
                                    msg_if_fail(
                                        result,
                                        "Failed to consume metafile",
                                        &mut gui.msg_dialog,
                                    );
                                }
                                if let Ok(off) = usize::from_str_radix(&self.filters.addr, 16) {
                                    let off = off - app.src_args.hard_seek.unwrap_or(0);
                                    app.edit_state.set_cursor(off);
                                    app.center_view_on_offset(off);
                                    app.hex_ui.flash_cursor();
                                }
                            }
                        });
                        row.col(|ui| {
                            let size = map_range.size();
                            let txt = size.to_string();
                            ui.add(egui::Label::new(&txt).sense(egui::Sense::click()))
                                .on_hover_text(human_size(size))
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
            ui.label(format!(
                "{}/{} maps shown ({})",
                filtered.len(),
                self.map_ranges.len(),
                crate::util::human_size(filtered.iter().map(|range| range.size()).sum::<usize>())
            ));
        } else {
            TableBuilder::new(ui)
                .column(Column::auto())
                .column(Column::remainder())
                .resizable(true)
                .striped(true)
                .header(20.0, |mut row| {
                    row.col(|ui| {
                        if sort_button(ui, "pid", true, self.pid_sort).clicked() {
                            self.pid_sort.flip()
                        }
                    });
                    row.col(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.filters.proc_name)
                                .hint_text("🔎 Name"),
                        );
                    });
                })
                .body(|body| {
                    let procs = self.sys.processes();
                    let filt_str = self.filters.proc_name.to_ascii_lowercase();
                    let mut pids: Vec<&sysinfo::Pid> = procs
                        .keys()
                        .filter(|&pid| {
                            procs[pid]
                                .name()
                                .to_string_lossy()
                                .to_ascii_lowercase()
                                .contains(&filt_str)
                        })
                        .collect();
                    pids.sort_by(|pid1, pid2| match self.pid_sort {
                        Sort::Ascending => pid1.cmp(pid2),
                        Sort::Descending => pid1.cmp(pid2).reverse(),
                    });
                    body.rows(20.0, pids.len(), |mut row| {
                        let pid = pids[row.index()];
                        row.col(|ui| {
                            if ui
                                .selectable_label(Some(*pid) == self.selected_pid, pid.to_string())
                                .clicked()
                            {
                                self.selected_pid = Some(*pid);
                                match pid.to_string().parse() {
                                    Ok(pid) => refresh_proc_maps(
                                        pid,
                                        &mut self.map_ranges,
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
                            ui.label(procs[pid].name().to_string_lossy());
                        });
                    });
                });
        }
    }

    fn title(&self) -> &str {
        "Open process"
    }
}

fn should_retain_range(filters: &Filters, range: &proc_maps::MapRange) -> bool {
    if filters.perms.read && !range.is_read() {
        return false;
    }
    if filters.perms.write && !range.is_write() {
        return false;
    }
    if filters.perms.execute && !range.is_exec() {
        return false;
    }
    if let Ok(addr) = usize::from_str_radix(&filters.addr, 16) {
        if !(range.start() <= addr && range.start() + range.size() >= addr) {
            return false;
        }
    }
    if filters.path.is_empty() {
        return true;
    }
    match range.filename() {
        Some(path) => path.display().to_string().contains(&filters.path),
        None => false,
    }
}

fn refresh_proc_maps(pid: u32, win_map_ranges: &mut MapRanges, msg: &mut MessageDialog) {
    #[cfg_attr(
        windows,
        expect(clippy::useless_conversion, reason = "lossless on windows")
    )]
    match proc_maps::get_process_maps(pid.try_into().expect("Couldnt't convert process id")) {
        Ok(ranges) => {
            *win_map_ranges = ranges;
        }
        Err(e) => msg_fail(&e, "Failed to get map ranges for process", msg),
    }
}
