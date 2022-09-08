pub mod edit_state;
pub mod interact_mode;
pub mod presentation;

use std::{
    ffi::OsString,
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::mpsc::Receiver,
    thread,
    time::{Duration, Instant},
};

use anyhow::{bail, Context};

use egui_sfml::sfml::graphics::Font;
use gamedebug_core::per_msg;
use slotmap::Key;

use crate::{
    args::{Args, SourceArgs},
    config::Config,
    gui::Gui,
    hex_ui::HexUi,
    input::Input,
    layout::{default_margin, do_auto_layout, Layout},
    meta::{
        perspective::Perspective, region::Region, LayoutKey, LayoutMap, Meta, NamedRegion,
        NamedView, PerspectiveKey, PerspectiveMap, RegionMap, ViewKey,
    },
    meta_state::MetaState,
    preferences::Preferences,
    shell::{msg_if_fail, msg_warn},
    source::{Source, SourceAttributes, SourcePermissions, SourceProvider, SourceState},
    timer::Timer,
    view::{HexData, TextData, View, ViewKind},
};

use self::edit_state::EditState;

/// The hexerator application state
pub struct App {
    pub data: Vec<u8>,
    pub edit_state: EditState,
    pub input: Input,
    pub args: Args,
    pub source: Option<Source>,
    pub just_reloaded: bool,
    pub stream_read_recv: Option<Receiver<Vec<u8>>>,
    pub cfg: Config,
    last_reload: Instant,
    pub preferences: Preferences,
    pub hex_ui: HexUi,
    pub meta_state: MetaState,
}

impl App {
    pub fn new(mut args: Args, mut cfg: Config, font: &Font) -> anyhow::Result<Self> {
        let mut data = Vec::new();
        let mut source = None;
        if args.recent && let Some(recent) = cfg.recent.most_recent() {
            args.src = recent.clone();
        }
        let load_success = load_file_from_src_args(&mut args.src, &mut cfg, &mut source, &mut data);
        let mut this = Self {
            data,
            edit_state: EditState::default(),
            input: Input::default(),
            args,
            source,
            just_reloaded: true,
            stream_read_recv: None,
            cfg,
            last_reload: Instant::now(),
            preferences: Preferences::default(),
            hex_ui: HexUi::default(),
            meta_state: MetaState::default(),
        };
        if load_success {
            this.new_file_readjust(font);
            if let Some(meta_path) = &this.args.meta {
                consume_meta_from_file(meta_path.clone(), &mut this)?;
            }
        }
        if let Some(offset) = this.args.src.jump {
            this.center_view_on_offset(offset);
            this.edit_state.cursor = offset;
            this.flash_cursor();
        }
        Ok(this)
    }
    pub fn reload(&mut self) -> anyhow::Result<()> {
        match &mut self.source {
            Some(src) => match &mut src.provider {
                SourceProvider::File(file) => {
                    self.data = read_contents(&self.args.src, file)?;
                    self.edit_state.dirty_region = None;
                }
                SourceProvider::Stdin(_) => {
                    bail!("Can't reload streaming sources like standard input")
                }
            },
            None => bail!("No file to reload"),
        }
        self.just_reloaded = true;
        Ok(())
    }
    pub fn save(&mut self) -> anyhow::Result<()> {
        let file = match &mut self.source {
            Some(src) => match &mut src.provider {
                SourceProvider::File(file) => file,
                SourceProvider::Stdin(_) => bail!("Standard input doesn't support saving"),
            },
            None => bail!("No source opened, nothing to save"),
        };
        let offset = self.args.src.hard_seek.unwrap_or(0);
        file.seek(SeekFrom::Start(offset as u64))?;
        let data_to_write = match self.edit_state.dirty_region {
            Some(region) => {
                eprintln!(
                    "Writing dirty region {}..{}, size {}",
                    region.begin,
                    region.end,
                    // TODO: See below, same +1 stuff
                    (region.end - region.begin) + 1,
                );
                file.seek(SeekFrom::Current(region.begin as _))?;
                // TODO: We're assuming here that end of the region is the same position as the last dirty byte
                // Make sure to enforce this invariant.
                // Add 1 to the end to write the dirty region even if it's 1 byte
                &self.data[region.begin..region.end + 1]
            }
            None => &self.data,
        };
        file.write_all(data_to_write)?;
        self.edit_state.dirty_region = None;
        if let Err(e) = self.save_temp_metafile_backup() {
            per_msg!("Failed to save metafile backup: {}", e);
        }
        Ok(())
    }
    pub fn save_temp_metafile_backup(&mut self) -> anyhow::Result<()> {
        // We set the last_meta_backup first, so if save fails, we don't get
        // a never ending stream of constant save failures.
        self.meta_state.last_meta_backup.set(Instant::now());
        self.save_meta_to_file(temp_metafile_backup_path(), true)?;
        per_msg!("Saved temp metafile backup");
        Ok(())
    }
    pub fn toggle_debug(&mut self) {
        gamedebug_core::toggle();
    }
    pub fn search_focus(&mut self, offset: usize) {
        self.edit_state.cursor = offset;
        self.center_view_on_offset(offset);
        self.flash_cursor();
    }

    pub(crate) fn center_view_on_offset(&mut self, offset: usize) {
        if let Some(key) = self.hex_ui.focused_view {
            self.meta_state.meta.views[key].view.center_on_offset(
                offset,
                &self.meta_state.meta.perspectives,
                &self.meta_state.meta.regions,
            );
        }
    }

    pub(crate) fn backup_path(&self) -> Option<PathBuf> {
        self.args.src.file.as_ref().map(|file| {
            let mut os_string = OsString::from(file);
            os_string.push(".hexerator_bak");
            os_string.into()
        })
    }
    pub(crate) fn dec_cols(&mut self) {
        self.col_change_impl(|col| *col -= 1);
    }
    fn col_change_impl(&mut self, f: impl FnOnce(&mut usize)) {
        if let Some(key) = self.hex_ui.focused_view {
            let view = &mut self.meta_state.meta.views[key].view;
            col_change_impl_view_perspective(
                view,
                &mut self.meta_state.meta.perspectives,
                &self.meta_state.meta.regions,
                f,
                self.preferences.col_change_lock_col,
                self.preferences.col_change_lock_row,
            );
        }
    }
    pub(crate) fn inc_cols(&mut self) {
        self.col_change_impl(|col| *col += 1);
    }
    pub(crate) fn halve_cols(&mut self) {
        self.col_change_impl(|col| *col /= 2);
    }
    pub(crate) fn double_cols(&mut self) {
        self.col_change_impl(|col| *col *= 2);
    }
    pub fn cursor_history_back(&mut self) {
        if self.edit_state.cursor_history_back() {
            self.center_view_on_offset(self.edit_state.cursor);
            self.flash_cursor();
        }
    }
    pub fn cursor_history_forward(&mut self) {
        if self.edit_state.cursor_history_forward() {
            self.center_view_on_offset(self.edit_state.cursor);
            self.flash_cursor();
        }
    }

    pub(crate) fn load_file(
        &mut self,
        path: PathBuf,
        read_only: bool,
        font: &Font,
    ) -> Result<(), anyhow::Error> {
        self.load_file_args(
            Args {
                src: SourceArgs {
                    file: Some(path),
                    jump: None,
                    hard_seek: None,
                    take: None,
                    read_only,
                    stream: false,
                },
                recent: false,
                meta: None,
            },
            font,
        )
    }

    /// Readjust to a new file
    fn new_file_readjust(&mut self, font: &Font) {
        self.meta_state.meta = Meta::default();
        self.meta_state.current_meta_path.clear();
        let def_region = self.meta_state.meta.regions.insert(NamedRegion {
            name: "default".into(),
            region: Region {
                begin: 0,
                end: self.data.len().saturating_sub(1),
            },
            desc: String::new(),
        });
        let default_perspective = self.meta_state.meta.perspectives.insert(Perspective {
            region: def_region,
            cols: 48,
            flip_row_order: false,
            name: "default".to_string(),
        });
        let mut layout = Layout {
            name: "Default layout".into(),
            view_grid: vec![vec![]],
            margin: default_margin(),
        };
        for view in default_views(font, default_perspective) {
            let k = self.meta_state.meta.views.insert(view);
            layout.view_grid[0].push(k);
        }
        let layout_key = self.meta_state.meta.layouts.insert(layout);
        App::switch_layout(
            &mut self.hex_ui.current_layout,
            &mut self.hex_ui.focused_view,
            &self.meta_state.meta.layouts,
            layout_key,
        );
    }

    pub fn close_file(&mut self) {
        // We potentially had large data, free it instead of clearing the Vec
        self.data = Vec::new();
        self.args.src.file = None;
        self.source = None;
    }

    pub(crate) fn restore_backup(&mut self) -> Result<(), anyhow::Error> {
        std::fs::copy(
            &self.backup_path().context("Failed to get backup path")?,
            self.args.src.file.as_ref().context("No file open")?,
        )?;
        self.reload()
    }

    pub(crate) fn create_backup(&self) -> Result<(), anyhow::Error> {
        std::fs::copy(
            self.args.src.file.as_ref().context("No file open")?,
            &self.backup_path().context("Failed to get backup path")?,
        )?;
        Ok(())
    }

    pub(crate) fn set_cursor_init(&mut self) {
        self.edit_state.cursor = self.args.src.jump.unwrap_or(0);
        self.center_view_on_offset(self.edit_state.cursor);
        self.flash_cursor();
    }
    pub fn flash_cursor(&mut self) {
        self.hex_ui.flash_cursor_timer = Timer::set(Duration::from_millis(1500));
    }
    /// If the cursor should be flashing, returns a timer value that can be used to color cursor
    pub fn cursor_flash_timer(app_flash_cursor_timer: &Timer) -> Option<u32> {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "
        The duration will never be higher than u32 limit.

        It doesn't make sense to set the cursor timer to extremely high values,
        only a few seconds at most.
        "
        )]
        app_flash_cursor_timer
            .overtime()
            .map(|dur| dur.as_millis() as u32)
    }

    pub(crate) fn try_read_stream(&mut self) {
        let Some(src) = &mut self.source else { return };
        if !src.attr.stream {
            return;
        };
        let Some(view_key) = self.hex_ui.focused_view else { return };
        let view = &self.meta_state.meta.views[view_key].view;
        let view_byte_offset = view
            .offsets(
                &self.meta_state.meta.perspectives,
                &self.meta_state.meta.regions,
            )
            .byte;
        let bytes_per_page = view.bytes_per_page(&self.meta_state.meta.perspectives);
        // Don't read past what we need for our current view offset
        if view_byte_offset + bytes_per_page < self.data.len() {
            return;
        }
        if src.state.stream_end {
            return;
        }
        match &self.stream_read_recv {
            Some(recv) => match recv.try_recv() {
                Ok(buf) => {
                    if buf.is_empty() {
                        src.state.stream_end = true;
                    } else {
                        self.data.extend_from_slice(&buf[..]);
                        let perspective = &self.meta_state.meta.perspectives[view.perspective];
                        let region = &mut self.meta_state.meta.regions[perspective.region].region;
                        region.end = self.data.len() - 1;
                    }
                }
                Err(e) => match e {
                    std::sync::mpsc::TryRecvError::Empty => {}
                    std::sync::mpsc::TryRecvError::Disconnected => self.stream_read_recv = None,
                },
            },
            None => {
                let (tx, rx) = std::sync::mpsc::channel();
                let mut src_clone = src.provider.clone();
                self.stream_read_recv = Some(rx);
                thread::spawn(move || {
                    let buffer_size = 1024;
                    let mut buf = vec![0; buffer_size];
                    let result: anyhow::Result<()> = try {
                        let amount = src_clone.read(&mut buf)?;
                        buf.truncate(amount);
                        tx.send(buf)?;
                    };
                    if let Err(e) = result {
                        msg_warn(&format!("Stream error: {}", e));
                    }
                });
            }
        }
    }
    // Byte offset of a pixel position in the viewport
    //
    // Also returns the index of the view the position is from
    pub fn byte_offset_at_pos(&mut self, x: i16, y: i16) -> Option<(usize, ViewKey)> {
        let layout = &self.meta_state.meta.layouts[self.hex_ui.current_layout];
        for view_key in layout.iter() {
            let view = &self.meta_state.meta.views[view_key];
            if let Some((row, col)) = view.view.row_col_offset_of_pos(
                x,
                y,
                &self.meta_state.meta.perspectives,
                &self.meta_state.meta.regions,
            ) {
                return Some((
                    self.meta_state.meta.perspectives[view.view.perspective]
                        .byte_offset_of_row_col(row, col, &self.meta_state.meta.regions),
                    view_key,
                ));
            }
        }
        None
    }
    pub fn view_idx_at_pos(&self, x: i16, y: i16) -> Option<ViewKey> {
        let layout = &self.meta_state.meta.layouts[self.hex_ui.current_layout];
        for view_key in layout.iter() {
            let view = &self.meta_state.meta.views[view_key];
            if view.view.viewport_rect.contains_pos(x, y) {
                return Some(view_key);
            }
        }
        None
    }

    pub fn save_meta_to_file(&mut self, path: PathBuf, temp: bool) -> Result<(), anyhow::Error> {
        let data = rmp_serde::to_vec(&self.meta_state.meta)?;
        std::fs::write(&path, &data)?;
        if !temp {
            self.meta_state.current_meta_path = path;
            self.meta_state.clean_meta = self.meta_state.meta.clone();
        }
        Ok(())
    }

    pub(crate) fn load_file_args(&mut self, mut args: Args, font: &Font) -> anyhow::Result<()> {
        if load_file_from_src_args(
            &mut args.src,
            &mut self.cfg,
            &mut self.source,
            &mut self.data,
        ) {
            self.args = args;
        }
        if !self.preferences.keep_meta {
            self.new_file_readjust(font);
        }
        Ok(())
    }
    /// Called every frame
    pub(crate) fn update(&mut self) {
        if !self.hex_ui.current_layout.is_null() {
            let layout = &self.meta_state.meta.layouts[self.hex_ui.current_layout];
            do_auto_layout(
                layout,
                &mut self.meta_state.meta.views,
                &self.hex_ui.hex_iface_rect,
                &self.meta_state.meta.perspectives,
                &self.meta_state.meta.regions,
            );
        }
        if self.preferences.auto_save && self.edit_state.dirty_region.is_some() {
            if let Err(e) = self.save() {
                per_msg!("Save fail: {}", e);
            }
        }
        if self.preferences.auto_reload
            && self.last_reload.elapsed().as_millis()
                >= u128::from(self.preferences.auto_reload_interval_ms)
        {
            if msg_if_fail(self.reload(), "Auto-reload fail").is_some() {
                self.preferences.auto_reload = false;
            }
            self.last_reload = Instant::now();
        }
    }
    /// Returns the selection marked by select_a and select_b
    pub(crate) fn selection(
        app_select_a: &Option<usize>,
        app_select_b: &Option<usize>,
    ) -> Option<Region> {
        if let Some(a) = app_select_a && let Some(b) = app_select_b {
            Some(Region {
                begin: *a.min(b),
                end: *a.max(b),
            })
        } else {
            None
        }
    }
    pub(crate) fn focused_view_select_all(&mut self) {
        if let Some(view) = self.hex_ui.focused_view {
            let p_key = self.meta_state.meta.views[view].view.perspective;
            let p = &self.meta_state.meta.perspectives[p_key];
            let r = &self.meta_state.meta.regions[p.region];
            self.hex_ui.select_a = Some(r.region.begin);
            self.hex_ui.select_b = Some(r.region.end);
        }
    }

    pub(crate) fn source_file(&self) -> Option<&Path> {
        self.args.src.file.as_deref()
    }

    pub(crate) fn diff_with_file(&mut self, path: PathBuf, gui: &mut Gui) -> anyhow::Result<()> {
        let file_data = read_source_to_buf(&path, &self.args.src)?;
        let mut diff_entries = Vec::new();
        for ((offset, &my_byte), &file_byte) in self.data.iter().enumerate().zip(file_data.iter()) {
            if my_byte != file_byte {
                diff_entries.push(FileDiffEntry {
                    my_val: my_byte,
                    file_val: file_byte,
                    offset,
                });
            }
        }
        gui.file_diff_result_window.diff_entries = diff_entries;
        gui.file_diff_result_window.path = path;
        gui.file_diff_result_window.open.set(true);
        Ok(())
    }

    pub(crate) fn switch_layout(
        app_current_layout: &mut LayoutKey,
        app_focused_view: &mut Option<ViewKey>,
        app_meta_layouts: &LayoutMap,
        k: LayoutKey,
    ) {
        *app_current_layout = k;
        // Set focused view to the first available view in the layout
        if let Some(view_key) = app_meta_layouts[k]
            .view_grid
            .get(0)
            .and_then(|row| row.get(0))
        {
            *app_focused_view = Some(*view_key);
        }
    }
    /// Clear existing meta references, in anticipation for loading new metafile
    fn clear_meta_refs(&mut self) {
        self.hex_ui.current_layout = LayoutKey::null();
        self.hex_ui.focused_view = None;
    }

    pub(crate) fn focus_prev_view_in_layout(&mut self) {
        if let Some(focused_view_key) = self.hex_ui.focused_view {
            let layout = &self.meta_state.meta.layouts[self.hex_ui.current_layout];
            if let Some(focused_idx) = layout.iter().position(|k| k == focused_view_key) {
                let new_idx = if focused_idx == 0 {
                    layout.iter().count() - 1
                } else {
                    focused_idx - 1
                };
                if let Some(new_key) = layout.iter().nth(new_idx) {
                    self.hex_ui.focused_view = Some(new_key);
                }
            }
        }
    }

    pub(crate) fn focus_next_view_in_layout(&mut self) {
        if let Some(focused_view_key) = self.hex_ui.focused_view {
            let layout = &self.meta_state.meta.layouts[self.hex_ui.current_layout];
            if let Some(focused_idx) = layout.iter().position(|k| k == focused_view_key) {
                let new_idx = if focused_idx == layout.iter().count() - 1 {
                    0
                } else {
                    focused_idx + 1
                };
                if let Some(new_key) = layout.iter().nth(new_idx) {
                    self.hex_ui.focused_view = Some(new_key);
                }
            }
        }
    }

    pub(crate) fn load_proc_memory(
        &mut self,
        pid: sysinfo::Pid,
        start: usize,
        size: usize,
        is_write: bool,
        font: &Font,
    ) -> anyhow::Result<()> {
        #[cfg(target_os = "linux")]
        return load_proc_memory_linux(self, pid, start, size, is_write, font);
        #[cfg(not(target_os = "linux"))]
        bail!("Only supported on linux right now. Sorry!")
    }

    pub(crate) fn start_offset_of_view(
        view: &mut View,
        perspectives: &PerspectiveMap,
        regions: &RegionMap,
    ) -> usize {
        let p = &perspectives[view.perspective];
        regions[p.region].region.begin
    }

    pub(crate) fn end_offset_of_view(
        view: &mut View,
        perspectives: &PerspectiveMap,
        regions: &RegionMap,
    ) -> usize {
        let p = &perspectives[view.perspective];
        regions[p.region].region.end
    }
}

#[cfg(target_os = "linux")]
fn load_proc_memory_linux(
    app: &mut App,
    pid: sysinfo::Pid,
    start: usize,
    size: usize,
    is_write: bool,
    font: &Font,
) -> anyhow::Result<()> {
    app.load_file_args(
        Args {
            src: SourceArgs {
                file: Some(Path::new("/proc/").join(pid.to_string()).join("mem")),
                jump: None,
                hard_seek: Some(start),
                take: Some(size),
                read_only: !is_write,
                stream: false,
            },
            recent: false,
            meta: None,
        },
        font,
    )
}

pub fn read_source_to_buf(path: &Path, args: &SourceArgs) -> Result<Vec<u8>, anyhow::Error> {
    let mut f = std::fs::File::open(path)?;
    if let &Some(to) = &args.hard_seek {
        f.seek(std::io::SeekFrom::Current(to as i64))?;
    }
    #[expect(
        clippy::cast_possible_truncation,
        reason = "On 32 bit, max supported file size is 4 GB"
    )]
    let len = args.take.unwrap_or(f.metadata()?.len() as usize);
    let mut buf = vec![0; len];
    f.read_exact(&mut buf)?;
    Ok(buf)
}

pub struct FileDiffEntry {
    pub my_val: u8,
    pub file_val: u8,
    pub offset: usize,
}

pub fn temp_metafile_backup_path() -> PathBuf {
    std::env::temp_dir().join("hexerator_meta_backup.meta")
}

pub fn consume_meta_from_file(path: PathBuf, this: &mut App) -> Result<(), anyhow::Error> {
    let data = std::fs::read(&path)?;
    this.clear_meta_refs();
    this.meta_state.meta = rmp_serde::from_slice(&data)?;
    this.meta_state.clean_meta = this.meta_state.meta.clone();
    this.meta_state.current_meta_path = path;
    this.meta_state.meta.post_load_init();
    // Switch to first layout, if there is one
    if let Some(layout_key) = this.meta_state.meta.layouts.keys().next() {
        App::switch_layout(
            &mut this.hex_ui.current_layout,
            &mut this.hex_ui.focused_view,
            &this.meta_state.meta.layouts,
            layout_key,
        );
    }
    Ok(())
}

pub fn col_change_impl_view_perspective(
    view: &mut View,
    perspectives: &mut PerspectiveMap,
    regions: &RegionMap,
    f: impl FnOnce(&mut usize),
    lock_x: bool,
    lock_y: bool,
) {
    let prev_offset = view.offsets(perspectives, regions);
    f(&mut perspectives[view.perspective].cols);
    perspectives[view.perspective].clamp_cols(regions);
    view.scroll_to_byte_offset(prev_offset.byte, perspectives, regions, lock_x, lock_y);
}

pub fn default_views(font: &Font, perspective: PerspectiveKey) -> Vec<NamedView> {
    vec![
        NamedView {
            view: View::new(ViewKind::Hex(HexData::default()), perspective),
            name: "Default hex".into(),
        },
        NamedView {
            view: View::new(
                ViewKind::Text(TextData::default_from_font(font, 14)),
                perspective,
            ),
            name: "Default text".into(),
        },
        NamedView {
            view: View::new(ViewKind::Block, perspective),
            name: "Default block".into(),
        },
    ]
}

/// Returns if the file was actually loaded.
fn load_file_from_src_args(
    src_args: &mut SourceArgs,
    cfg: &mut Config,
    source: &mut Option<Source>,
    data: &mut Vec<u8>,
) -> bool {
    if let Some(file_arg) = &src_args.file {
        if file_arg.as_os_str() == "-" {
            *source = Some(Source {
                provider: SourceProvider::Stdin(std::io::stdin()),
                attr: SourceAttributes {
                    seekable: false,
                    stream: true,
                    permissions: SourcePermissions {
                        read: true,
                        write: false,
                    },
                },
                state: SourceState::default(),
            });
            true
        } else {
            let result: Result<(), anyhow::Error> = try {
                let mut file = open_file(file_arg, src_args.read_only)?;
                data.clear();
                if let Some(path) = &mut src_args.file {
                    match path.canonicalize() {
                        Ok(canon) => *path = canon,
                        Err(e) => msg_warn(&format!(
                            "Failed to canonicalize path {}: {}\n\
                             Recent use list might not be able to load it back.",
                            path.display(),
                            e
                        )),
                    }
                }
                cfg.recent.use_(src_args.clone());
                if !src_args.stream {
                    *data = read_contents(&*src_args, &mut file)?;
                }
                *source = Some(Source {
                    provider: SourceProvider::File(file),
                    attr: SourceAttributes {
                        seekable: true,
                        stream: src_args.stream,
                        permissions: SourcePermissions {
                            read: true,
                            write: !src_args.read_only,
                        },
                    },
                    state: SourceState::default(),
                });
            };
            match result {
                Ok(()) => true,
                Err(e) => {
                    msg_warn(&format!("Failed to open file: {}", e));
                    false
                }
            }
        }
    } else {
        false
    }
}

fn open_file(path: &Path, read_only: bool) -> Result<File, anyhow::Error> {
    OpenOptions::new()
        .read(true)
        .write(!read_only)
        .open(path)
        .context("Failed to open file")
}

fn read_contents(args: &SourceArgs, file: &mut File) -> anyhow::Result<Vec<u8>> {
    let seek = args.hard_seek.unwrap_or(0);
    file.seek(SeekFrom::Start(seek as u64))?;
    let mut data = Vec::new();
    match args.take {
        Some(amount) => (&*file).take(amount as u64).read_to_end(&mut data)?,
        None => file.read_to_end(&mut data)?,
    };
    Ok(data)
}
