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
        gui::{
            file_diff_result_window::FileDiffResultWindow,
            message_dialog::{Icon, MessageDialog},
            Gui,
        },
        hex_ui::HexUi,
        input::Input,
        layout::{default_margin, do_auto_layout, Layout},
        meta::{
            perspective::Perspective, region::Region, LayoutKey, Meta, NamedRegion, NamedView,
            PerspectiveKey, PerspectiveMap, RegionKey, RegionMap, ViewKey,
        },
        meta_state::MetaState,
        plugin::PluginContainer,
        preferences::{Autoreload, Preferences},
        shell::{msg_fail, msg_if_fail},
        source::{Source, SourceAttributes, SourcePermissions, SourceProvider, SourceState},
        view::{HexData, TextData, View, ViewKind, ViewportScalar},
    },
    anyhow::{bail, Context},
    egui_commonmark::CommonMarkCache,
    egui_sfml::sfml::graphics::{Font, RenderWindow},
    gamedebug_core::{per, per_dbg},
    hexerator_plugin_api::MethodResult,
    mlua::Lua,
    slotmap::Key,
    std::{
        ffi::OsString,
        fs::{File, OpenOptions},
        io::{Read, Seek, SeekFrom, Write},
        path::{Path, PathBuf},
        sync::mpsc::Receiver,
        thread,
        time::Instant,
    },
};

mod backend_command;
pub mod command;
pub mod edit_state;
pub mod interact_mode;
pub mod presentation;

/// The hexerator application state
pub struct App {
    pub data: Vec<u8>,
    /// Original data length. Compared with current data length to detect truncation.
    pub orig_data_len: usize,
    pub edit_state: EditState,
    pub input: Input,
    pub src_args: SourceArgs,
    pub source: Option<Source>,
    pub just_reloaded: bool,
    stream_read_recv: Option<Receiver<Vec<u8>>>,
    pub cfg: Config,
    last_reload: Instant,
    pub preferences: Preferences,
    pub hex_ui: HexUi,
    pub meta_state: MetaState,
    pub clipboard: arboard::Clipboard,
    pub md_cache: CommonMarkCache,
    /// Command queue for queuing up operations to perform on the application state.
    pub cmd: CommandQueue,
    pub backend_cmd: BackendCommandQueue,
    /// A quit was requested
    pub quit_requested: bool,
    pub plugins: Vec<PluginContainer>,
}

impl App {
    pub(crate) fn new(
        mut args: Args,
        cfg: Config,
        font: &Font,
        msg: &mut MessageDialog,
    ) -> anyhow::Result<Self> {
        if args.recent
            && let Some(recent) = cfg.recent.most_recent()
        {
            args.src = recent.clone();
        }
        let mut this = Self {
            orig_data_len: 0,
            data: Vec::new(),
            edit_state: EditState::default(),
            input: Input::default(),
            src_args: SourceArgs::default(),
            source: None,
            just_reloaded: true,
            stream_read_recv: None,
            cfg,
            last_reload: Instant::now(),
            preferences: Preferences::default(),
            hex_ui: HexUi::default(),
            meta_state: MetaState::default(),
            clipboard: arboard::Clipboard::new()?,
            md_cache: CommonMarkCache::default(),
            cmd: Default::default(),
            backend_cmd: Default::default(),
            quit_requested: false,
            plugins: Vec::new(),
        };
        for path in args.load_plugin {
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
        // Set a clean meta, for an empty document
        this.set_new_clean_meta(font);
        msg_if_fail(
            this.load_file_args(args.src, args.meta, font, msg),
            "Failed to load file",
            msg,
        );
        if let Some(name) = args.layout {
            if !Self::switch_layout_by_name(&mut this.hex_ui, &this.meta_state.meta, &name) {
                let err = anyhow::anyhow!("No layout with name '{name}' found.");
                msg_fail(&err, "Couldn't switch layout", msg);
            }
        }
        Ok(this)
    }
    pub fn reload(&mut self) -> anyhow::Result<()> {
        match &mut self.source {
            Some(src) => match &mut src.provider {
                SourceProvider::File(file) => {
                    self.data = read_contents(&self.src_args, file)?;
                    self.edit_state.dirty_region = None;
                }
                SourceProvider::Stdin(_) => {
                    bail!("Can't reload streaming sources like standard input")
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
            None => bail!("No file to reload"),
        }
        self.just_reloaded = true;
        Ok(())
    }
    pub fn save(&mut self, msg: &mut MessageDialog) -> anyhow::Result<()> {
        let file = match &mut self.source {
            Some(src) => match &mut src.provider {
                SourceProvider::File(file) => file,
                SourceProvider::Stdin(_) => bail!("Standard input doesn't support saving"),
                #[cfg(windows)]
                SourceProvider::WinProc { handle, start, .. } => {
                    if let Some(region) = self.edit_state.dirty_region {
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
                                bail!("Failed to write process memory");
                            }
                        }
                        self.edit_state.dirty_region = None;
                    }
                    return Ok(());
                }
            },
            None => bail!("No surce opened, nothing to save"),
        };
        // If the file was truncated, we completely save over it
        if self.data.len() != self.orig_data_len {
            msg.open(
                Icon::Warn,
                "File truncated/extended",
                "Data is truncated/extended. Are you sure you want to save?",
            );
            msg.custom_button_row_ui(Box::new(|ui, modal, cmd| {
                if ui
                    .button(egui::RichText::new("Save & Truncate").color(egui::Color32::RED))
                    .clicked()
                {
                    modal.close();
                    cmd.push(Cmd::SaveTruncateFinish);
                }
                if ui.button("Cancel").clicked() {
                    modal.close();
                }
            }));
            return Ok(());
        }
        let offset = self.src_args.hard_seek.unwrap_or(0);
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
        self.edit_state.dirty_region = None;
        if let Err(e) = self.save_temp_metafile_backup() {
            per!("Failed to save metafile backup: {}", e);
        }
        Ok(())
    }
    pub fn save_truncated_file_finish(&mut self) -> anyhow::Result<()> {
        let Some(source) = &mut self.source else {
            bail!("There is no source");
        };
        let SourceProvider::File(file) = &mut source.provider else {
            bail!("Source is not a file");
        };
        file.set_len(self.data.len() as u64)?;
        file.rewind()?;
        file.write_all(&self.data)?;
        self.edit_state.dirty_region = None;
        self.orig_data_len = self.data.len();
        Ok(())
    }
    pub fn save_temp_metafile_backup(&mut self) -> anyhow::Result<()> {
        // We set the last_meta_backup first, so if save fails, we don't get
        // a never ending stream of constant save failures.
        self.meta_state.last_meta_backup.set(Instant::now());
        self.save_meta_to_file(temp_metafile_backup_path(), true)?;
        per!("Saved temp metafile backup");
        Ok(())
    }
    pub fn search_focus(&mut self, offset: usize) {
        self.edit_state.cursor = offset;
        self.center_view_on_offset(offset);
        self.hex_ui.flash_cursor();
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

    pub(crate) fn center_view_on_offset(&mut self, offset: usize) {
        if let Some(key) = self.hex_ui.focused_view {
            self.meta_state.meta.views[key].view.center_on_offset(
                offset,
                &self.meta_state.meta.low.perspectives,
                &self.meta_state.meta.low.regions,
            );
        }
    }

    pub(crate) fn backup_path(&self) -> Option<PathBuf> {
        self.src_args.file.as_ref().map(|file| {
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
                &mut self.meta_state.meta.low.perspectives,
                &self.meta_state.meta.low.regions,
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
            self.hex_ui.flash_cursor();
        }
    }
    pub fn cursor_history_forward(&mut self) {
        if self.edit_state.cursor_history_forward() {
            self.center_view_on_offset(self.edit_state.cursor);
            self.hex_ui.flash_cursor();
        }
    }

    pub(crate) fn load_file(
        &mut self,
        path: PathBuf,
        read_only: bool,
        font: &Font,
        msg: &mut MessageDialog,
    ) -> Result<(), anyhow::Error> {
        self.load_file_args(
            SourceArgs {
                file: Some(path),
                jump: None,
                hard_seek: None,
                take: None,
                read_only,
                stream: false,
            },
            None,
            font,
            msg,
        )
    }

    /// Set a new clean meta for the current data, and switch to default layout
    pub fn set_new_clean_meta(&mut self, font: &Font) {
        per!("Setting up new clean meta");
        self.meta_state.current_meta_path.clear();
        self.meta_state.meta = Meta::default();
        let layout_key = setup_empty_meta(self.data.len(), font, &mut self.meta_state.meta);
        self.meta_state.clean_meta = self.meta_state.meta.clone();
        App::switch_layout(&mut self.hex_ui, &self.meta_state.meta, layout_key);
    }

    pub fn close_file(&mut self) {
        // We potentially had large data, free it instead of clearing the Vec
        self.data = Vec::new();
        self.src_args.file = None;
        self.source = None;
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

    pub(crate) fn set_cursor_init(&mut self) {
        self.edit_state.cursor = self.src_args.jump.unwrap_or(0);
        self.center_view_on_offset(self.edit_state.cursor);
        self.hex_ui.flash_cursor();
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
                        per!("Stream error: {}", e);
                    }
                });
            }
        }
    }
    // Byte offset of a pixel position in the viewport
    //
    // Also returns the index of the view the position is from
    pub fn byte_offset_at_pos(&self, x: i16, y: i16) -> Option<(usize, ViewKey)> {
        let layout = self
            .meta_state
            .meta
            .layouts
            .get(self.hex_ui.current_layout)?;
        for view_key in layout.iter() {
            if let Some(pos) = self.view_byte_offset_at_pos(view_key, x, y) {
                return Some((pos, view_key));
            }
        }
        None
    }
    pub fn view_byte_offset_at_pos(&self, view_key: ViewKey, x: i16, y: i16) -> Option<usize> {
        let view = &self.meta_state.meta.views[view_key].view;
        view.row_col_offset_of_pos(
            x,
            y,
            &self.meta_state.meta.low.perspectives,
            &self.meta_state.meta.low.regions,
        )
        .map(|(row, col)| {
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

    pub fn save_meta_to_file(&mut self, path: PathBuf, temp: bool) -> Result<(), anyhow::Error> {
        let data = rmp_serde::to_vec(&self.meta_state.meta)?;
        std::fs::write(&path, data)?;
        if !temp {
            self.meta_state.current_meta_path = path;
            self.meta_state.clean_meta = self.meta_state.meta.clone();
        }
        Ok(())
    }

    pub(crate) fn load_file_args(
        &mut self,
        mut src_args: SourceArgs,
        meta_path: Option<PathBuf>,
        font: &Font,
        msg: &mut MessageDialog,
    ) -> anyhow::Result<()> {
        if load_file_from_src_args(
            &mut src_args,
            &mut self.cfg,
            &mut self.source,
            &mut self.data,
            msg,
            &mut self.cmd,
        ) {
            // Loaded new file, set the "original" data length to this length to prepare for truncation/etc.
            self.orig_data_len = self.data.len();
            // Set up meta
            if !self.preferences.keep_meta {
                if let Some(meta_path) = meta_path {
                    if let Err(e) = self.consume_meta_from_file(meta_path.clone()) {
                        self.set_new_clean_meta(font);
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
                        self.consume_meta_from_file(meta_path.clone())?;
                    }
                } else {
                    // We didn't load any meta, but we're loading a new file.
                    // Set up a new clean meta for it.
                    self.set_new_clean_meta(font);
                }
            }
            self.src_args = src_args;
            if let Some(offset) = self.src_args.jump {
                self.center_view_on_offset(offset);
                self.edit_state.cursor = offset;
                self.hex_ui.flash_cursor();
            }
        }
        Ok(())
    }
    /// Called every frame
    pub(crate) fn update(&mut self, gui: &mut Gui, rw: &mut RenderWindow, lua: &Lua, font: &Font) {
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
        if self.preferences.auto_save && self.edit_state.dirty_region.is_some() {
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
        self.flush_command_queue(gui, lua, font);
        self.flush_backend_command_queue(rw);
    }
    /// Reload only what's visible on the screen (current layout)
    fn reload_visible(&mut self) -> anyhow::Result<()> {
        let (lo, hi) = self.visible_byte_range();
        self.reload_range(lo, hi)
    }
    fn reload_range(&mut self, lo: usize, hi: usize) -> anyhow::Result<()> {
        let Some(src) = &self.source else {
            anyhow::bail!("No source")
        };
        anyhow::ensure!(lo <= hi);
        match &src.provider {
            SourceProvider::File(ref file) => {
                let mut file = file;
                let offset = match self.src_args.hard_seek {
                    Some(hs) => hs + lo,
                    None => lo,
                };
                file.seek(SeekFrom::Start(offset as u64))?;
                file.read_exact(&mut self.data[lo..hi])?;
                Ok(())
            }
            SourceProvider::Stdin(_) => anyhow::bail!("Not implemented"),
        }
    }
    /// Iterator over the views in the current layout
    fn active_views(&self) -> impl Iterator<Item = &'_ NamedView> {
        let layout = &self.meta_state.meta.layouts[self.hex_ui.current_layout];
        layout.iter().map(|key| &self.meta_state.meta.views[key])
    }
    /// Largest visible byte range in the current perspective
    fn visible_byte_range(&self) -> (usize, usize) {
        let mut min_lo = self.data.len();
        let mut max_hi = 0;
        for view in self.active_views() {
            let offsets = view.view.offsets(
                &self.meta_state.meta.low.perspectives,
                &self.meta_state.meta.low.regions,
            );
            let lo = offsets.byte;
            min_lo = std::cmp::min(min_lo, lo);
            let hi = lo
                + view
                    .view
                    .bytes_per_page(&self.meta_state.meta.low.perspectives);
            max_hi = std::cmp::max(max_hi, hi);
        }
        (
            min_lo.clamp(0, self.data.len()),
            max_hi.clamp(0, self.data.len()),
        )
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

    pub(crate) fn source_file(&self) -> Option<&Path> {
        self.src_args.file.as_deref()
    }

    pub(crate) fn diff_with_file(
        &mut self,
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

    pub(crate) fn switch_layout(app_hex_ui: &mut HexUi, app_meta: &Meta, k: LayoutKey) {
        app_hex_ui.current_layout = k;
        // Set focused view to the first available view in the layout
        if let Some(view_key) = app_meta.layouts[k]
            .view_grid
            .first()
            .and_then(|row| row.first())
        {
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
        msg: &mut MessageDialog,
    ) -> anyhow::Result<()> {
        #[cfg(target_os = "linux")]
        return load_proc_memory_linux(self, pid, start, size, is_write, font, msg);
        #[cfg(windows)]
        return crate::windows::load_proc_memory(self, pid, start, size, is_write, font);
        #[cfg(target_os = "macos")]
        return load_proc_memory_macos(self, pid, start, size, is_write, font, msg);
    }

    pub fn consume_meta_from_file(&mut self, path: PathBuf) -> Result<(), anyhow::Error> {
        per!("Consuming metafile: {}", path.display());
        let data = std::fs::read(&path)?;
        let meta = rmp_serde::from_slice(&data)?;
        self.hex_ui.clear_meta_refs();
        self.meta_state.meta = meta;
        self.meta_state.clean_meta = self.meta_state.meta.clone();
        self.meta_state.current_meta_path = path;
        self.meta_state.meta.post_load_init();
        // Switch to first layout, if there is one
        if let Some(layout_key) = self.meta_state.meta.layouts.keys().next() {
            App::switch_layout(&mut self.hex_ui, &self.meta_state.meta, layout_key);
        }
        Ok(())
    }

    pub fn focused_perspective<'a>(hex_ui: &HexUi, meta: &'a Meta) -> Option<&'a Perspective> {
        hex_ui.focused_view.map(|view_key| {
            let per_key = meta.views[view_key].view.perspective;
            &meta.low.perspectives[per_key]
        })
    }

    pub(crate) fn region_key_for_view(&self, view_key: ViewKey) -> RegionKey {
        let per_key = self.meta_state.meta.views[view_key].view.perspective;
        self.meta_state.meta.low.perspectives[per_key].region
    }

    pub fn add_perspective_from_region(&mut self, region_key: RegionKey, name: String) {
        let mut per = Perspective::from_region(region_key, name);
        if let Some(focused_per) = App::focused_perspective(&self.hex_ui, &self.meta_state.meta) {
            per.cols = focused_per.cols;
        }
        self.meta_state.meta.low.perspectives.insert(per);
    }

    pub(crate) fn zero_fill_region(&mut self, region: Region) {
        let range = region.begin..=region.end;
        if let Some(data) = self.data.get_mut(range.clone()) {
            data.fill(0);
            self.edit_state
                .widen_dirty_region(DamageRegion::RangeInclusive(range));
        }
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
}

/// Set up an empty meta with the defaults
pub fn setup_empty_meta(data_len: usize, font: &Font, meta: &mut Meta) -> LayoutKey {
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
    for view in default_views(font, default_perspective) {
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
    font: &Font,
    msg: &mut MessageDialog,
) -> anyhow::Result<()> {
    app.load_file_args(
        SourceArgs {
            file: Some(Path::new("/proc/").join(pid.to_string()).join("mem")),
            jump: None,
            hard_seek: Some(start),
            take: Some(size),
            read_only: !is_write,
            stream: false,
        },
        None,
        font,
        msg,
    )
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
    let mut f = std::fs::File::open(path)?;
    if let &Some(to) = &args.hard_seek {
        #[expect(
            clippy::cast_possible_wrap,
            reason = "Files bigger than i64::MAX aren't supported"
        )]
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
            let result: Result<(), anyhow::Error> = try {
                let mut file = open_file(file_arg, src_args.read_only)?;
                data.clear();
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
                    *data = read_contents(&*src_args, &mut file)?;
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
                    msg_fail(&e, "Failed to open file", msg);
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
