use {
    self::{
        backend_command::BackendCommandQueue,
        command::{Cmd, CommandQueue},
        edit_state::EditState,
    },
    crate::{
        args::{Args, SourceArgs},
        config::Config,
        damage_region::DamageRegion,
        data::Data,
        gui::{
            Gui,
            message_dialog::{Icon, MessageDialog},
            windows::FileDiffResultWindow,
        },
        hex_ui::HexUi,
        input::Input,
        layout::{Layout, default_margin, do_auto_layout},
        meta::{
            LayoutKey, Meta, NamedRegion, NamedView, PerspectiveKey, PerspectiveMap, RegionKey,
            RegionMap, ViewKey, perspective::Perspective, region::Region,
        },
        meta_state::MetaState,
        plugin::PluginContainer,
        session_prefs::{Autoreload, SessionPrefs},
        shell::{msg_fail, msg_if_fail},
        source::{Source, SourceAttributes, SourcePermissions, SourceProvider, SourceState},
        view::{HexData, TextData, View, ViewKind, ViewportScalar},
    },
    anyhow::Context as _,
    egui_sf2g::sf2g::graphics::RenderWindow,
    gamedebug_core::{per, per_dbg},
    hexerator_plugin_api::MethodResult,
    mlua::Lua,
    slotmap::Key as _,
    std::{
        ffi::OsString,
        fs::{File, OpenOptions},
        io::{Read as _, Seek as _, SeekFrom, Write as _},
        path::{Path, PathBuf},
        sync::mpsc::Receiver,
        thread,
        time::Instant,
    },
};

pub mod backend_command;
pub mod command;
mod debug;
pub mod edit_state;
pub mod interact_mode;
pub mod presentation;

/// The hexerator application state
pub struct App {
    pub data: Data,
    pub edit_state: EditState,
    pub input: Input,
    pub src_args: SourceArgs,
    pub source: Option<Source>,
    stream_read_recv: Option<Receiver<Vec<u8>>>,
    pub cfg: Config,
    last_reload: Instant,
    pub preferences: SessionPrefs,
    pub hex_ui: HexUi,
    pub meta_state: MetaState,
    pub clipboard: arboard::Clipboard,
    /// Command queue for queuing up operations to perform on the application state.
    pub cmd: CommandQueue,
    pub backend_cmd: BackendCommandQueue,
    /// A quit was requested
    pub quit_requested: bool,
    pub plugins: Vec<PluginContainer>,
    /// Size of the buffer used for streaming reads
    pub stream_buffer_size: usize,
}

const DEFAULT_STREAM_BUFFER_SIZE: usize = 65_536;

/// Source management
impl App {
    pub fn reload(&mut self) -> anyhow::Result<()> {
        match &mut self.source {
            Some(src) => match &mut src.provider {
                SourceProvider::File(file) => {
                    self.data.reload_from_file(&self.src_args, file)?;
                }
                SourceProvider::Stdin(_) => {
                    anyhow::bail!("Can't reload streaming sources like standard input")
                }
                #[cfg(windows)]
                SourceProvider::WinProc {
                    handle,
                    start,
                    size,
                } => unsafe {
                    crate::windows::read_proc_memory(*handle, &mut self.data, *start, *size)?;
                },
            },
            None => anyhow::bail!("No file to reload"),
        }
        Ok(())
    }
    pub(crate) fn load_file_args(
        &mut self,
        mut src_args: SourceArgs,
        meta_path: Option<PathBuf>,
        msg: &mut MessageDialog,
        font_size: u16,
        line_spacing: u16,
    ) {
        if load_file_from_src_args(
            &mut src_args,
            &mut self.cfg,
            &mut self.source,
            &mut self.data,
            msg,
            &mut self.cmd,
        ) {
            // Set up meta
            if !self.preferences.keep_meta {
                if let Some(meta_path) = meta_path {
                    if let Err(e) = self.consume_meta_from_file(meta_path, false) {
                        self.set_new_clean_meta(font_size, line_spacing);
                        msg_fail(&e, "Failed to load metafile", msg);
                    }
                } else if let Some(src_path) = per_dbg!(&src_args.file)
                    && let Some(meta_path) = per_dbg!(self.cfg.meta_assocs.get(src_path))
                {
                    // We only load if the new meta path is not the same as the old.
                    // Keep the current metafile otherwise
                    if self.meta_state.current_meta_path != *meta_path {
                        per!(
                            "Mismatch: {:?} vs. {:?}",
                            self.meta_state.current_meta_path.display(),
                            meta_path.display()
                        );
                        let meta_path = meta_path.clone();
                        if let Err(e) = self.consume_meta_from_file(meta_path.clone(), false) {
                            self.set_new_clean_meta(font_size, line_spacing);
                            msg_fail(&e, &format!("Failed to load metafile {meta_path:?}"), msg);
                        }
                    }
                } else {
                    // We didn't load any meta, but we're loading a new file.
                    // Set up a new clean meta for it.
                    self.set_new_clean_meta(font_size, line_spacing);
                }
            }
            self.src_args = src_args;
            if let Some(offset) = self.src_args.jump {
                self.center_view_on_offset(offset);
                self.edit_state.cursor = offset;
                self.hex_ui.flash_cursor();
            }
        }
    }
    pub fn save(&mut self, msg: &mut MessageDialog) -> anyhow::Result<()> {
        let file = match &mut self.source {
            Some(src) => match &mut src.provider {
                SourceProvider::File(file) => file,
                SourceProvider::Stdin(_) => anyhow::bail!("Standard input doesn't support saving"),
                #[cfg(windows)]
                SourceProvider::WinProc { handle, start, .. } => {
                    if let Some(region) = self.data.dirty_region {
                        let mut n_write = 0;
                        unsafe {
                            if windows_sys::Win32::System::Diagnostics::Debug::WriteProcessMemory(
                                *handle,
                                (*start + region.begin) as _,
                                self.data[region.begin..].as_mut_ptr() as _,
                                region.len(),
                                &mut n_write,
                            ) == 0
                            {
                                anyhow::bail!("Failed to write process memory");
                            }
                        }
                        self.data.dirty_region = None;
                    }
                    return Ok(());
                }
            },
            None => anyhow::bail!("No surce opened, nothing to save"),
        };
        // If the file was truncated, we completely save over it
        if self.data.len() != self.data.orig_data_len {
            msg.open(
                Icon::Warn,
                "File truncated/extended",
                "Data is truncated/extended. Are you sure you want to save?",
            );
            msg.custom_button_row_ui(Box::new(|ui, payload, cmd| {
                if ui
                    .button(egui::RichText::new("Save & Truncate").color(egui::Color32::RED))
                    .clicked()
                {
                    payload.close = true;
                    cmd.push(Cmd::SaveTruncateFinish);
                }
                if ui.button("Cancel").clicked() {
                    payload.close = true;
                }
            }));
            return Ok(());
        }
        let offset = self.src_args.hard_seek.unwrap_or(0);
        file.seek(SeekFrom::Start(offset as u64))?;
        let data_to_write = match self.data.dirty_region {
            Some(region) => {
                #[expect(
                    clippy::cast_possible_wrap,
                    reason = "Files bigger than i64::MAX aren't supported"
                )]
                file.seek(SeekFrom::Current(region.begin as _))?;
                // TODO: We're assuming here that end of the region is the same position as the last dirty byte
                // Make sure to enforce this invariant.
                // Add 1 to the end to write the dirty region even if it's 1 byte
                self.data.get(region.begin..region.end + 1)
            }
            None => Some(&self.data[..]),
        };
        let Some(data_to_write) = data_to_write else {
            anyhow::bail!("No data to write (possibly out of bounds indexing)");
        };
        file.write_all(data_to_write)?;
        self.data.undirty();
        if let Err(e) = self.save_temp_metafile_backup() {
            per!("Failed to save metafile backup: {}", e);
        }
        Ok(())
    }
    pub fn save_truncated_file_finish(&mut self) -> anyhow::Result<()> {
        let Some(source) = &mut self.source else {
            anyhow::bail!("There is no source");
        };
        let SourceProvider::File(file) = &mut source.provider else {
            anyhow::bail!("Source is not a file");
        };
        file.set_len(self.data.len() as u64)?;
        file.rewind()?;
        file.write_all(&self.data)?;
        self.data.undirty();
        Ok(())
    }
    pub(crate) fn source_file(&self) -> Option<&Path> {
        self.src_args.file.as_deref()
    }
    pub(crate) fn load_file(
        &mut self,
        path: PathBuf,
        read_only: bool,
        msg: &mut MessageDialog,
        font_size: u16,
        line_spacing: u16,
    ) {
        self.load_file_args(
            SourceArgs {
                file: Some(path),
                jump: None,
                hard_seek: None,
                take: None,
                read_only,
                stream: false,
                stream_buffer_size: None,
                unsafe_mmap: None,
                mmap_len: None,
            },
            None,
            msg,
            font_size,
            line_spacing,
        );
    }

    pub fn close_file(&mut self) {
        // We potentially had large data, free it instead of clearing the Vec
        self.data.close();
        self.src_args.file = None;
        self.source = None;
    }

    pub(crate) fn backup_path(&self) -> Option<PathBuf> {
        self.src_args.file.as_ref().map(|file| {
            let mut os_string = OsString::from(file);
            os_string.push(".hexerator_bak");
            os_string.into()
        })
    }

    pub(crate) fn restore_backup(&mut self) -> Result<(), anyhow::Error> {
        std::fs::copy(
            self.backup_path().context("Failed to get backup path")?,
            self.src_args.file.as_ref().context("No file open")?,
        )?;
        self.reload()
    }

    pub(crate) fn create_backup(&self) -> Result<(), anyhow::Error> {
        std::fs::copy(
            self.src_args.file.as_ref().context("No file open")?,
            self.backup_path().context("Failed to get backup path")?,
        )?;
        Ok(())
    }
    /// Reload only what's visible on the screen (current layout)
    fn reload_visible(&mut self) -> anyhow::Result<()> {
        let [lo, hi] = self.visible_byte_range();
        self.reload_range(lo, hi)
    }
    pub fn reload_range(&mut self, lo: usize, hi: usize) -> anyhow::Result<()> {
        let Some(src) = &self.source else {
            anyhow::bail!("No source")
        };
        anyhow::ensure!(lo <= hi);
        match &src.provider {
            SourceProvider::File(file) => {
                let mut file = file;
                let offset = match self.src_args.hard_seek {
                    Some(hs) => hs + lo,
                    None => lo,
                };
                file.seek(SeekFrom::Start(offset as u64))?;
                match self.data.get_mut(lo..=hi) {
                    Some(buf) => file.read_exact(buf)?,
                    None => anyhow::bail!("Reload range out of bounds"),
                }
                Ok(())
            }
            SourceProvider::Stdin(_) => anyhow::bail!("Not implemented"),
            #[cfg(windows)]
            SourceProvider::WinProc { .. } => anyhow::bail!("Not implemented"),
        }
    }
    #[allow(clippy::unnecessary_wraps, reason = "cfg shenanigans")]
    pub(crate) fn load_proc_memory(
        &mut self,
        pid: sysinfo::Pid,
        start: usize,
        size: usize,
        is_write: bool,
        msg: &mut MessageDialog,
        font_size: u16,
        line_spacing: u16,
    ) -> anyhow::Result<()> {
        #[cfg(target_os = "linux")]
        {
            load_proc_memory_linux(
                self,
                pid,
                start,
                size,
                is_write,
                msg,
                font_size,
                line_spacing,
            );
            Ok(())
        }
        #[cfg(windows)]
        return crate::windows::load_proc_memory(
            self,
            pid,
            start,
            size,
            is_write,
            font_size,
            line_spacing,
            msg,
        );
        #[cfg(target_os = "macos")]
        return load_proc_memory_macos(self, pid, start, size, is_write, font, msg);
    }
}

/// Metafile
impl App {
    /// Set a new clean meta for the current data, and switch to default layout
    pub fn set_new_clean_meta(&mut self, font_size: u16, line_spacing: u16) {
        per!("Setting up new clean meta");
        self.meta_state.current_meta_path.clear();
        self.meta_state.meta = Meta::default();
        let layout_key = setup_empty_meta(
            self.data.len(),
            &mut self.meta_state.meta,
            font_size,
            line_spacing,
        );
        self.meta_state.clean_meta = self.meta_state.meta.clone();
        Self::switch_layout(&mut self.hex_ui, &self.meta_state.meta, layout_key);
    }
    /// Like `set_new_clean_meta`, but keeps the clean meta intact
    ///
    /// Used for "Clear meta" action.
    pub fn clear_meta(&mut self, font_size: u16, line_spacing: u16) {
        self.meta_state.meta = Meta::default();
        let layout_key = setup_empty_meta(
            self.data.len(),
            &mut self.meta_state.meta,
            font_size,
            line_spacing,
        );
        Self::switch_layout(&mut self.hex_ui, &self.meta_state.meta, layout_key);
    }
    pub fn save_temp_metafile_backup(&mut self) -> anyhow::Result<()> {
        // We set the last_meta_backup first, so if save fails, we don't get
        // a never ending stream of constant save failures.
        self.meta_state.last_meta_backup.set(Instant::now());
        self.save_meta_to_file(temp_metafile_backup_path(), true)?;
        per!("Saved temp metafile backup");
        Ok(())
    }
    pub fn save_meta_to_file(&mut self, path: PathBuf, temp: bool) -> Result<(), anyhow::Error> {
        let data = rmp_serde::to_vec(&self.meta_state.meta)?;
        std::fs::write(&path, data)?;
        if !temp {
            self.meta_state.current_meta_path = path;
            self.meta_state.clean_meta = self.meta_state.meta.clone();
        }
        Ok(())
    }
    pub fn save_meta(&mut self) -> Result<(), anyhow::Error> {
        self.save_meta_to_file(self.meta_state.current_meta_path.clone(), false)
    }
    pub fn consume_meta_from_file(
        &mut self,
        path: PathBuf,
        temp: bool,
    ) -> Result<(), anyhow::Error> {
        per!("Consuming metafile: {}", path.display());
        let data = std::fs::read(&path)?;
        let meta = rmp_serde::from_slice(&data).context("Deserialization error")?;
        self.hex_ui.clear_meta_refs();
        self.meta_state.meta = meta;
        if !temp {
            self.meta_state.current_meta_path = path;
            self.meta_state.clean_meta = self.meta_state.meta.clone();
        }
        self.meta_state.meta.post_load_init();
        // Switch to first layout, if there is one
        if let Some(layout_key) = self.meta_state.meta.layouts.keys().next() {
            Self::switch_layout(&mut self.hex_ui, &self.meta_state.meta, layout_key);
        }
        Ok(())
    }

    pub fn add_perspective_from_region(
        &mut self,
        region_key: RegionKey,
        name: String,
    ) -> PerspectiveKey {
        let mut per = Perspective::from_region(region_key, name);
        if let Some(focused_per) = Self::focused_perspective(&self.hex_ui, &self.meta_state.meta) {
            per.cols = focused_per.cols;
        }
        self.meta_state.meta.low.perspectives.insert(per)
    }
}

/// Navigation
impl App {
    pub fn search_focus(&mut self, offset: usize) {
        self.edit_state.cursor = offset;
        self.center_view_on_offset(offset);
        self.hex_ui.flash_cursor();
    }
    pub(crate) fn center_view_on_offset(&mut self, offset: usize) {
        if let Some(key) = self.hex_ui.focused_view {
            self.meta_state.meta.views[key].view.center_on_offset(
                offset,
                &self.meta_state.meta.low.perspectives,
                &self.meta_state.meta.low.regions,
            );
        }
    }
    pub fn cursor_history_back(&mut self) {
        if self.edit_state.cursor_history_back() {
            self.center_view_on_offset(self.edit_state.cursor);
            self.hex_ui.flash_cursor();
        }
    }
    pub fn cursor_history_forward(&mut self) {
        if self.edit_state.cursor_history_forward() {
            self.center_view_on_offset(self.edit_state.cursor);
            self.hex_ui.flash_cursor();
        }
    }

    pub(crate) fn set_cursor_init(&mut self) {
        self.edit_state.cursor = self.src_args.jump.unwrap_or(0);
        self.center_view_on_offset(self.edit_state.cursor);
        self.hex_ui.flash_cursor();
    }
    pub(crate) fn switch_layout(app_hex_ui: &mut HexUi, app_meta: &Meta, k: LayoutKey) {
        app_hex_ui.current_layout = k;
        // Set focused view to the first available view in the layout
        if let Some(view_key) = app_meta.layouts[k].view_grid.first().and_then(|row| row.first()) {
            app_hex_ui.focused_view = Some(*view_key);
        }
    }
    /// Tries to switch to a layout with the given name. Returns `false` if a layout with that name wasn't found.
    #[must_use]
    pub(crate) fn switch_layout_by_name(
        app_hex_ui: &mut HexUi,
        app_meta: &Meta,
        name: &str,
    ) -> bool {
        match app_meta.layouts.iter().find(|(_k, v)| v.name == name) {
            Some((k, _v)) => {
                Self::switch_layout(app_hex_ui, app_meta, k);
                true
            }
            None => false,
        }
    }

    /// Tries to focus a view with the given name. Returns `false` if a view with that name wasn't found.
    #[must_use]
    pub(crate) fn focus_first_view_of_name(
        app_hex_ui: &mut HexUi,
        app_meta: &Meta,
        name: &str,
    ) -> bool {
        match app_meta.views.iter().find(|(_k, v)| v.name == name) {
            Some((k, _v)) => {
                Self::focus_first_view_of_key(app_hex_ui, app_meta, k);
                true
            }
            None => false,
        }
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

    pub(crate) fn focus_first_view_of_key(
        app_hex_ui: &mut HexUi,
        app_meta: &Meta,
        view_key: ViewKey,
    ) {
        if let Some(layout_key) = app_meta
            .layouts
            .iter()
            .find_map(|(k, l)| l.contains_view(view_key).then_some(k))
        {
            Self::switch_layout(app_hex_ui, app_meta, layout_key);
            app_hex_ui.focused_view = Some(view_key);
        }
    }
}

/// Perspective manipulation
impl App {
    pub(crate) fn inc_cols(&mut self) {
        self.col_change_impl(|col| *col += 1);
    }
    pub(crate) fn dec_cols(&mut self) {
        self.col_change_impl(|col| *col -= 1);
    }
    pub(crate) fn halve_cols(&mut self) {
        self.col_change_impl(|col| *col /= 2);
    }
    pub(crate) fn double_cols(&mut self) {
        self.col_change_impl(|col| *col *= 2);
    }
    fn col_change_impl(&mut self, f: impl FnOnce(&mut usize)) {
        if let Some(key) = self.hex_ui.focused_view {
            let view = &mut self.meta_state.meta.views[key].view;
            col_change_impl_view_perspective(
                view,
                &mut self.meta_state.meta.low.perspectives,
                &self.meta_state.meta.low.regions,
                f,
                self.preferences.col_change_lock_col,
                self.preferences.col_change_lock_row,
            );
        }
    }
}

/// Finding things
impl App {
    // Byte offset of a pixel position in the viewport
    //
    // Also returns the index of the view the position is from
    pub fn byte_offset_at_pos(&self, x: i16, y: i16) -> Option<(usize, ViewKey)> {
        let layout = self.meta_state.meta.layouts.get(self.hex_ui.current_layout)?;
        for view_key in layout.iter() {
            if let Some(pos) = self.view_byte_offset_at_pos(view_key, x, y) {
                return Some((pos, view_key));
            }
        }
        None
    }
    pub fn view_byte_offset_at_pos(&self, view_key: ViewKey, x: i16, y: i16) -> Option<usize> {
        let NamedView { view, .. } = self.meta_state.meta.views.get(view_key)?;
        view.row_col_offset_of_pos(
            x,
            y,
            &self.meta_state.meta.low.perspectives,
            &self.meta_state.meta.low.regions,
        )
        .map(|[row, col]| {
            self.meta_state.meta.low.perspectives[view.perspective].byte_offset_of_row_col(
                row,
                col,
                &self.meta_state.meta.low.regions,
            )
        })
    }
    pub fn view_at_pos(&self, x: ViewportScalar, y: ViewportScalar) -> Option<ViewKey> {
        let layout = &self.meta_state.meta.layouts[self.hex_ui.current_layout];
        for row in &layout.view_grid {
            for key in row {
                let view = &self.meta_state.meta.views[*key];
                if view.view.viewport_rect.contains_pos(x, y) {
                    return Some(*key);
                }
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
    /// Iterator over the views in the current layout
    fn active_views(&self) -> impl Iterator<Item = &'_ NamedView> {
        let layout = &self.meta_state.meta.layouts[self.hex_ui.current_layout];
        layout.iter().map(|key| &self.meta_state.meta.views[key])
    }
    /// Largest visible byte range in the current perspective
    fn visible_byte_range(&self) -> [usize; 2] {
        let mut min_lo = self.data.len();
        let mut max_hi = 0;
        for view in self.active_views() {
            let offsets = view.view.offsets(
                &self.meta_state.meta.low.perspectives,
                &self.meta_state.meta.low.regions,
            );
            let lo = offsets.byte;
            min_lo = std::cmp::min(min_lo, lo);
            let hi = lo + view.view.bytes_per_page(&self.meta_state.meta.low.perspectives);
            max_hi = std::cmp::max(max_hi, hi);
        }
        [min_lo, max_hi].map(|v| v.clamp(0, self.data.len()))
    }
    pub(crate) fn focused_view_mut(&mut self) -> Option<(ViewKey, &mut View)> {
        self.hex_ui.focused_view.and_then(|key| {
            self.meta_state.meta.views.get_mut(key).map(|view| (key, &mut view.view))
        })
    }
    pub(crate) fn row_region(&self, row: usize) -> Option<Region> {
        let per = Self::focused_perspective(&self.hex_ui, &self.meta_state.meta)?;
        let per_reg = self.meta_state.meta.low.regions.get(per.region)?.region;
        // Beginning of the region
        let beg = per_reg.begin;
        // Number of columns
        let cols = per.cols;
        let row_begin = beg + row * cols;
        // Regions are inclusive, so we subtract 1
        let row_end = (row_begin + cols).saturating_sub(1);
        Some(Region {
            begin: row_begin,
            end: row_end,
        })
    }

    pub(crate) fn col_offsets(&self, col: usize) -> Option<Vec<usize>> {
        let per = Self::focused_perspective(&self.hex_ui, &self.meta_state.meta)?;
        let per_reg = self.meta_state.meta.low.regions.get(per.region)?.region;
        let beg = per_reg.begin;
        let end = per_reg.end;
        let cols = per.cols;
        let offsets = (beg..=end).step_by(cols).map(|off| off + col).collect();
        Some(offsets)
    }

    pub(crate) fn cursor_col_offsets(&self) -> Option<Vec<usize>> {
        self.row_col_of_cursor().and_then(|[_, col]| self.col_offsets(col))
    }
    /// Returns the row and column of the provided byte position, according to focused perspective
    pub(crate) fn row_col_of_byte_pos(&self, pos: usize) -> Option<[usize; 2]> {
        Self::focused_perspective(&self.hex_ui, &self.meta_state.meta)
            .map(|per| calc_perspective_row_col(pos, per, &self.meta_state.meta.low.regions))
    }
    /// Returns the byte position of the provided row and column, according to focused perspective
    pub(crate) fn byte_pos_of_row_col(&self, row: usize, col: usize) -> Option<usize> {
        Self::focused_perspective(&self.hex_ui, &self.meta_state.meta).map(|per| {
            calc_perspective_row_col_offset(row, col, per, &self.meta_state.meta.low.regions)
        })
    }
    /// Returns the row and column of the current cursor, according to focused perspective
    pub(crate) fn row_col_of_cursor(&self) -> Option<[usize; 2]> {
        self.row_col_of_byte_pos(self.edit_state.cursor)
    }
    pub fn focused_perspective<'a>(hex_ui: &HexUi, meta: &'a Meta) -> Option<&'a Perspective> {
        hex_ui.focused_view.map(|view_key| {
            let per_key = meta.views[view_key].view.perspective;
            &meta.low.perspectives[per_key]
        })
    }
    pub fn focused_region<'a>(hex_ui: &HexUi, meta: &'a Meta) -> Option<&'a NamedRegion> {
        Self::focused_perspective(hex_ui, meta).and_then(|per| meta.low.regions.get(per.region))
    }

    pub(crate) fn region_key_for_view(&self, view_key: ViewKey) -> RegionKey {
        let per_key = self.meta_state.meta.views[view_key].view.perspective;
        self.meta_state.meta.low.perspectives[per_key].region
    }
    /// Figure out the byte offset of the row `offset` is on
    pub(crate) fn find_row_start(&self, offset: usize) -> Option<usize> {
        match self.row_col_of_byte_pos(offset) {
            Some([row, _col]) => self.byte_pos_of_row_col(row, 0),
            None => None,
        }
    }
    /// Figure out the byte offset of the row `offset` is on + end
    pub(crate) fn find_row_end(&self, offset: usize) -> Option<usize> {
        Self::focused_perspective(&self.hex_ui, &self.meta_state.meta).map(|per| {
            let [row, _col] =
                calc_perspective_row_col(offset, per, &self.meta_state.meta.low.regions);
            calc_perspective_row_col_offset(
                row,
                per.cols.saturating_sub(1),
                per,
                &self.meta_state.meta.low.regions,
            )
        })
    }
}

fn calc_perspective_row_col(pos: usize, per: &Perspective, regions: &RegionMap) -> [usize; 2] {
    let cols = per.cols;
    let region_begin = regions[per.region].region.begin;
    let byte_pos = pos.saturating_sub(region_begin);
    [byte_pos / cols, byte_pos % cols]
}

fn calc_perspective_row_col_offset(
    row: usize,
    col: usize,
    per: &Perspective,
    regions: &RegionMap,
) -> usize {
    let region_begin = regions[per.region].region.begin;
    row * per.cols + col + region_begin
}

/// Editing
impl App {
    pub(crate) fn mod_byte_at_cursor(&mut self, f: impl FnOnce(&mut u8)) {
        if let Some(byte) = self.data.get_mut(self.edit_state.cursor) {
            f(byte);
            self.data.widen_dirty_region(DamageRegion::Single(self.edit_state.cursor));
        }
    }

    pub(crate) fn inc_byte_at_cursor(&mut self) {
        self.mod_byte_at_cursor(|b| *b = b.wrapping_add(1));
    }

    pub(crate) fn dec_byte_at_cursor(&mut self) {
        self.mod_byte_at_cursor(|b| *b = b.wrapping_sub(1));
    }

    pub(crate) fn inc_byte_or_bytes(&mut self) {
        let mut any = false;
        for region in self.hex_ui.selected_regions() {
            self.data.mod_range(region.to_range(), |byte| {
                *byte = byte.wrapping_add(1);
            });
            any = true;
        }
        if !any {
            self.inc_byte_at_cursor();
        }
    }

    pub(crate) fn dec_byte_or_bytes(&mut self) {
        let mut any = false;
        for region in self.hex_ui.selected_regions() {
            self.data.mod_range(region.to_range(), |byte| {
                *byte = byte.wrapping_sub(1);
            });
            any = true;
        }
        if !any {
            self.dec_byte_at_cursor();
        }
    }
}

/// Etc.
impl App {
    pub(crate) fn new(
        mut args: Args,
        cfg: Config,
        font_size: u16,
        line_spacing: u16,
        msg: &mut MessageDialog,
    ) -> anyhow::Result<Self> {
        if args.recent
            && let Some(recent) = cfg.recent.most_recent()
        {
            args.src = recent.clone();
        }
        let mut this = Self {
            data: Data::default(),
            edit_state: EditState::default(),
            input: Input::default(),
            src_args: SourceArgs::default(),
            source: None,
            stream_read_recv: None,
            cfg,
            last_reload: Instant::now(),
            preferences: SessionPrefs::default(),
            hex_ui: HexUi::default(),
            meta_state: MetaState::default(),
            clipboard: arboard::Clipboard::new()?,
            cmd: Default::default(),
            backend_cmd: Default::default(),
            quit_requested: false,
            plugins: Vec::new(),
            stream_buffer_size: args.src.stream_buffer_size.unwrap_or(DEFAULT_STREAM_BUFFER_SIZE),
        };
        for path in args.load_plugin {
            // Safety: This will cause UB on a bad plugin. Nothing we can do.
            //
            // It's up to the user not to load bad plugins.
            this.plugins.push(unsafe { PluginContainer::new(path)? });
        }
        if args.autosave {
            this.preferences.auto_save = true;
        }
        if let Some(interval_ms) = args.autoreload {
            if args.autoreload_only_visible {
                this.preferences.auto_reload = Autoreload::Visible;
            } else {
                this.preferences.auto_reload = Autoreload::All;
            }
            this.preferences.auto_reload_interval_ms = interval_ms;
        }
        match args.new {
            Some(new_len) => {
                if let Some(path) = args.src.file {
                    if path.exists() {
                        anyhow::bail!("Can't use --new for {path:?}: File already exists");
                    }
                    // Set up source for this new file
                    let f = OpenOptions::new()
                        .create(true)
                        .truncate(false)
                        .read(true)
                        .write(true)
                        .open(&path)?;
                    f.set_len(new_len as u64)?;
                    this.source = Some(Source::file(f));
                    this.src_args.file = Some(path);
                }
                this.data = Data::clean_from_buf(vec![0; new_len]);
                // Set clean meta for the newly allocated buffer
                this.set_new_clean_meta(font_size, line_spacing);
            }
            None => {
                // Set a clean meta, for an empty document
                this.set_new_clean_meta(font_size, line_spacing);
                this.load_file_args(args.src, args.meta, msg, font_size, line_spacing);
            }
        }
        if let Some(name) = args.layout {
            if !Self::switch_layout_by_name(&mut this.hex_ui, &this.meta_state.meta, &name) {
                let err = anyhow::anyhow!("No layout with name '{name}' found.");
                msg_fail(&err, "Couldn't switch layout", msg);
            }
        }
        if let Some(name) = args.view {
            if !Self::focus_first_view_of_name(&mut this.hex_ui, &this.meta_state.meta, &name) {
                let err = anyhow::anyhow!("No view with name '{name}' found.");
                msg_fail(&err, "Couldn't focus view", msg);
            }
        }
        // Set cursor to the beginning of the focused region we ended up with
        if let Some(reg) = Self::focused_region(&this.hex_ui, &this.meta_state.meta) {
            this.edit_state.cursor = reg.region.begin;
        }
        Ok(this)
    }
    /// Reoffset all bookmarks based on the difference between the cursor and `offset`
    pub(crate) fn reoffset_bookmarks_cursor_diff(&mut self, offset: usize) {
        #[expect(
            clippy::cast_possible_wrap,
            reason = "We assume that the offset is not greater than isize"
        )]
        let difference = self.edit_state.cursor as isize - offset as isize;
        for bm in &mut self.meta_state.meta.bookmarks {
            bm.offset = bm.offset.saturating_add_signed(difference);
        }
    }

    pub(crate) fn try_read_stream(&mut self) {
        let Some(src) = &mut self.source else { return };
        if !src.attr.stream {
            return;
        };
        let Some(view_key) = self.hex_ui.focused_view else {
            return;
        };
        let view = &self.meta_state.meta.views[view_key].view;
        let view_byte_offset = view
            .offsets(
                &self.meta_state.meta.low.perspectives,
                &self.meta_state.meta.low.regions,
            )
            .byte;
        let bytes_per_page = view.bytes_per_page(&self.meta_state.meta.low.perspectives);
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
                        let perspective = &self.meta_state.meta.low.perspectives[view.perspective];
                        let region =
                            &mut self.meta_state.meta.low.regions[perspective.region].region;
                        region.end = self.data.len().saturating_sub(1);
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
                let buffer_size = self.stream_buffer_size;
                thread::spawn(move || {
                    let mut buf = vec![0; buffer_size];
                    let result: anyhow::Result<()> = try {
                        let amount = src_clone.read(&mut buf)?;
                        buf.truncate(amount);
                        tx.send(buf)?;
                    };
                    if let Err(e) = result {
                        per!("Stream error: {}", e);
                    }
                });
            }
        }
    }

    /// Called every frame
    pub(crate) fn update(
        &mut self,
        gui: &mut Gui,
        rw: &mut RenderWindow,
        lua: &Lua,
        font_size: u16,
        line_spacing: u16,
    ) {
        if !self.hex_ui.current_layout.is_null() {
            let layout = &self.meta_state.meta.layouts[self.hex_ui.current_layout];
            do_auto_layout(
                layout,
                &mut self.meta_state.meta.views,
                &self.hex_ui.hex_iface_rect,
                &self.meta_state.meta.low.perspectives,
                &self.meta_state.meta.low.regions,
            );
        }
        if self.preferences.auto_save && self.data.dirty_region.is_some() {
            if let Err(e) = self.save(&mut gui.msg_dialog) {
                per!("Save fail: {}", e);
            }
        }
        if self.preferences.auto_reload.is_active()
            && self.source.is_some()
            && self.last_reload.elapsed().as_millis()
                >= u128::from(self.preferences.auto_reload_interval_ms)
        {
            match &self.preferences.auto_reload {
                Autoreload::Disabled => {}
                Autoreload::All => {
                    if msg_if_fail(self.reload(), "Auto-reload fail", &mut gui.msg_dialog).is_some()
                    {
                        self.preferences.auto_reload = Autoreload::Disabled;
                    }
                }
                Autoreload::Visible => {
                    if msg_if_fail(
                        self.reload_visible(),
                        "Auto-reload fail",
                        &mut gui.msg_dialog,
                    )
                    .is_some()
                    {
                        self.preferences.auto_reload = Autoreload::Disabled;
                    }
                }
            }
            self.last_reload = Instant::now();
        }
        // Here we perform all queued up `Command`s.
        self.flush_command_queue(gui, lua, font_size, line_spacing);
        self.flush_backend_command_queue(rw);
    }

    pub(crate) fn focused_view_select_all(&mut self) {
        if let Some(view) = self.hex_ui.focused_view {
            let p_key = self.meta_state.meta.views[view].view.perspective;
            let p = &self.meta_state.meta.low.perspectives[p_key];
            let r = &self.meta_state.meta.low.regions[p.region];
            self.hex_ui.select_a = Some(r.region.begin);
            // Don't select more than the data length, even if region is bigger
            self.hex_ui.select_b = Some(r.region.end.min(self.data.len().saturating_sub(1)));
        }
    }

    pub(crate) fn focused_view_select_row(&mut self) {
        if let Some([row, _]) = self.row_col_of_cursor()
            && let Some(reg) = self.row_region(row)
        {
            // To make behavior consistent with "select col", we clear all extra selections beforehand
            self.hex_ui.extra_selections.clear();
            self.hex_ui.select_a = Some(reg.begin);
            self.hex_ui.select_b = Some(reg.end);
        }
    }

    pub(crate) fn focused_view_select_col(&mut self) {
        let Some(offsets) = self.cursor_col_offsets() else {
            return;
        };
        self.hex_ui.extra_selections.clear();
        let mut offsets = offsets.into_iter();
        if let Some(off) = offsets.next() {
            self.hex_ui.select_a = Some(off);
            self.hex_ui.select_b = Some(off);
        }
        for col in offsets {
            self.hex_ui.extra_selections.push(Region {
                begin: col,
                end: col,
            });
        }
    }

    pub(crate) fn diff_with_file(
        &self,
        path: PathBuf,
        file_diff_result_window: &mut FileDiffResultWindow,
    ) -> anyhow::Result<()> {
        // FIXME: Skipping ignores changes to bookmarked values that happen later than the first
        // byte.
        let file_data = read_source_to_buf(&path, &self.src_args)?;
        let mut offs = Vec::new();
        let mut skip = 0;
        for ((offset, &my_byte), &file_byte) in self.data.iter().enumerate().zip(file_data.iter()) {
            if skip > 0 {
                skip -= 1;
                continue;
            }
            if my_byte != file_byte {
                offs.push(offset);
            }
            if let Some((_, bm)) =
                Meta::bookmark_for_offset(&self.meta_state.meta.bookmarks, offset)
            {
                skip = bm.value_type.byte_len() - 1;
            }
        }
        file_diff_result_window.offsets = offs;
        file_diff_result_window.file_data = file_data;
        file_diff_result_window.path = path;
        file_diff_result_window.open.set(true);
        Ok(())
    }

    pub(crate) fn call_plugin_method(
        &mut self,
        plugin_name: &str,
        method_name: &str,
        args: &[Option<hexerator_plugin_api::Value>],
    ) -> MethodResult {
        let mut plugins = std::mem::take(&mut self.plugins);
        let result = 'block: {
            for plugin in &mut plugins {
                if plugin_name == plugin.plugin.name() {
                    break 'block plugin.plugin.on_method_called(method_name, args, self);
                }
            }
            Err(format!("Plugin `{plugin_name}` not found."))
        };
        std::mem::swap(&mut self.plugins, &mut plugins);
        result
    }

    pub(crate) fn remove_dangling(&mut self) {
        self.meta_state.meta.remove_dangling();
        if self
            .hex_ui
            .focused_view
            .is_some_and(|key| !self.meta_state.meta.views.contains_key(key))
        {
            eprintln!("Unset dangling focused view");
            self.hex_ui.focused_view = None;
        }
    }
}

/// Set up an empty meta with the defaults
pub fn setup_empty_meta(
    data_len: usize,
    meta: &mut Meta,
    font_size: u16,
    line_spacing: u16,
) -> LayoutKey {
    let def_region = meta.low.regions.insert(NamedRegion {
        name: "default".into(),
        region: Region {
            begin: 0,
            end: data_len.saturating_sub(1),
        },
        desc: String::new(),
    });
    let default_perspective = meta.low.perspectives.insert(Perspective {
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
    for view in default_views(default_perspective, font_size, line_spacing) {
        let k = meta.views.insert(view);
        layout.view_grid[0].push(k);
    }
    meta.layouts.insert(layout)
}

pub fn get_clipboard_string(cb: &mut arboard::Clipboard, msg: &mut MessageDialog) -> String {
    match cb.get_text() {
        Ok(text) => text,
        Err(e) => {
            msg.open(
                Icon::Error,
                "Failed to get text from clipboard",
                e.to_string(),
            );
            String::new()
        }
    }
}

pub fn set_clipboard_string(cb: &mut arboard::Clipboard, msg: &mut MessageDialog, text: &str) {
    msg_if_fail(cb.set_text(text), "Failed to set clipboard text", msg);
}

#[cfg(target_os = "linux")]
fn load_proc_memory_linux(
    app: &mut App,
    pid: sysinfo::Pid,
    start: usize,
    size: usize,
    is_write: bool,
    msg: &mut MessageDialog,
    font_size: u16,
    line_spacing: u16,
) {
    app.load_file_args(
        SourceArgs {
            file: Some(Path::new("/proc/").join(pid.to_string()).join("mem")),
            jump: None,
            hard_seek: Some(start),
            take: Some(size),
            read_only: !is_write,
            stream: false,
            stream_buffer_size: None,
            unsafe_mmap: None,
            mmap_len: None,
        },
        None,
        msg,
        font_size,
        line_spacing,
    );
}

#[cfg(target_os = "macos")]
fn load_proc_memory_macos(
    app: &mut App,
    pid: sysinfo::Pid,
    start: usize,
    size: usize,
    is_write: bool,
    font: &Font,
    msg: &mut MessageDialog,
    events: &EventQueue,
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
        msg,
        events,
    )
}

pub fn read_source_to_buf(path: &Path, args: &SourceArgs) -> Result<Vec<u8>, anyhow::Error> {
    let mut f = File::open(path)?;
    if let &Some(to) = &args.hard_seek {
        #[expect(
            clippy::cast_possible_wrap,
            reason = "Files bigger than i64::MAX aren't supported"
        )]
        f.seek(SeekFrom::Current(to as i64))?;
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

pub fn temp_metafile_backup_path() -> PathBuf {
    std::env::temp_dir().join("hexerator_meta_backup.meta")
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

pub fn default_views(
    perspective: PerspectiveKey,
    font_size: u16,
    line_spacing: u16,
) -> Vec<NamedView> {
    vec![
        NamedView {
            view: View::new(
                ViewKind::Hex(HexData::with_font_size(font_size)),
                perspective,
            ),
            name: "Default hex".into(),
        },
        NamedView {
            view: View::new(
                ViewKind::Text(TextData::with_font_info(line_spacing, font_size)),
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
    data: &mut Data,
    msg: &mut MessageDialog,
    cmd: &mut CommandQueue,
) -> bool {
    if let Some(file_arg) = &src_args.file {
        if file_arg.as_os_str() == "-" {
            *source = Some(Source {
                provider: SourceProvider::Stdin(std::io::stdin()),
                attr: SourceAttributes {
                    stream: true,
                    permissions: SourcePermissions { write: false },
                },
                state: SourceState::default(),
            });
            cmd.push(Cmd::ProcessSourceChange);
            true
        } else {
            let result: std::io::Result<()> = try {
                let mut file = open_file(file_arg, src_args.read_only)?;
                if let Some(path) = &mut src_args.file {
                    match path.canonicalize() {
                        Ok(canon) => *path = canon,
                        Err(e) => msg.open(
                            Icon::Warn,
                            "Warning",
                            format!(
                                "Failed to canonicalize path {}: {}\n\
                             Recent use list might not be able to load it back.",
                                path.display(),
                                e
                            ),
                        ),
                    }
                }
                cfg.recent.use_(src_args.clone());
                if !src_args.stream {
                    if let Some(mmap_mode) = src_args.unsafe_mmap {
                        let mut opts = memmap2::MmapOptions::new();
                        if let Some(len) = src_args.mmap_len {
                            opts.len(len);
                        }
                        // Safety:
                        //
                        // Memory mapped file access cannot be made 100% safe, not much we can do here.
                        //
                        // The command line option is called `--unsafe-mmap` to reflect this.
                        *data = unsafe {
                            match mmap_mode {
                                crate::args::MmapMode::Cow => {
                                    Data::new_mmap_mut(opts.map_copy(&file)?)
                                }
                                crate::args::MmapMode::DangerousMut => {
                                    Data::new_mmap_mut(opts.map_mut(&file)?)
                                }
                                crate::args::MmapMode::Ro => Data::new_mmap_immut(opts.map(&file)?),
                            }
                        };
                    } else {
                        *data = Data::clean_from_buf(read_contents(&*src_args, &mut file)?);
                    }
                }
                *source = Some(Source {
                    provider: SourceProvider::File(file),
                    attr: SourceAttributes {
                        stream: src_args.stream,
                        permissions: SourcePermissions {
                            write: !src_args.read_only,
                        },
                    },
                    state: SourceState::default(),
                });
                cmd.push(Cmd::ProcessSourceChange);
            };
            match result {
                Ok(()) => true,
                Err(e) => {
                    if !src_args.read_only && e.kind() == std::io::ErrorKind::PermissionDenied {
                        eprintln!("Failed to open file: {e}. Retrying read-only.");
                        src_args.read_only = true;
                        return load_file_from_src_args(src_args, cfg, source, data, msg, cmd);
                    }
                    msg_fail(&e, "Failed to open file", msg);
                    false
                }
            }
        }
    } else {
        false
    }
}

fn open_file(path: &Path, read_only: bool) -> std::io::Result<File> {
    OpenOptions::new().read(true).write(!read_only).open(path)
}

pub(crate) fn read_contents(args: &SourceArgs, file: &mut File) -> std::io::Result<Vec<u8>> {
    let seek = args.hard_seek.unwrap_or(0);
    file.seek(SeekFrom::Start(seek as u64))?;
    let mut data = Vec::new();
    match args.take {
        Some(amount) => (&*file).take(amount as u64).read_to_end(&mut data)?,
        None => file.read_to_end(&mut data)?,
    };
    Ok(data)
}
