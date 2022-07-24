mod edit_state;
pub mod interact_mode;
mod layout;
pub mod perspective;
pub mod presentation;

use std::{
    ffi::OsString,
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::mpsc::Receiver,
    thread,
    time::Duration,
};

use anyhow::{bail, Context};
use rfd::MessageButtons;
use serde::{Deserialize, Serialize};

use crate::{
    args::Args,
    config::Config,
    damage_region::DamageRegion,
    input::Input,
    metafile::Metafile,
    msg_if_fail, msg_warn,
    region::Region,
    source::Source,
    timer::Timer,
    view::{ScrollOffset, View, ViewKind, ViewportRect, ViewportScalar},
};

use self::{
    edit_state::EditState, interact_mode::InteractMode, layout::Layout, perspective::Perspective,
    presentation::Presentation,
};

/// The hexerator application state
#[derive(Debug)]
pub struct App {
    /// The default perspective
    pub perspective: Perspective,
    pub dirty_region: Option<Region>,
    pub data: Vec<u8>,
    pub edit_state: EditState,
    pub input: Input,
    pub interact_mode: InteractMode,
    pub presentation: Presentation,
    // The value of the cursor on the previous frame. Used to determine when the cursor changes
    pub prev_frame_inspect_offset: usize,
    pub views: Vec<View>,
    pub focused_view: Option<usize>,
    pub ui: crate::ui::Ui,
    pub selection: Option<Region>,
    pub select_begin: Option<usize>,
    pub args: Args,
    pub source: Option<Source>,
    pub col_change_lock_x: bool,
    pub col_change_lock_y: bool,
    flash_cursor_timer: Timer,
    pub stream_end: bool,
    pub just_reloaded: bool,
    pub layout: Layout,
    pub regions: Vec<NamedRegion>,
    /// Whether metafile needs saving
    pub meta_dirty: bool,
    pub stream_read_recv: Option<Receiver<Vec<u8>>>,
    pub cfg: Config,
    /// Whether to scissor views when drawing them. Useful to disable when debugging rendering.
    pub scissor_views: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NamedRegion {
    pub name: String,
    pub region: Region,
}

impl App {
    pub fn new(
        mut args: Args,
        window_height: ViewportScalar,
        mut cfg: Config,
    ) -> anyhow::Result<Self> {
        let mut data = Vec::new();
        let mut source = None;
        if args.load_recent && let Some(recent) = cfg.recent.most_recent() {
            args = recent.clone();
        }
        load_file_from_args(&mut args, &mut cfg, &mut source, &mut data);
        let layout = Layout::new();
        let cursor = 0;
        let mut views = default_views(&layout, window_height);
        views[0].go_home();
        let mut this = Self {
            scissor_views: true,
            perspective: Perspective::default(),
            dirty_region: None,
            data,
            edit_state: EditState::default(),
            input: Input::default(),
            interact_mode: InteractMode::View,
            presentation: Presentation::default(),
            // The value of the cursor on the previous frame. Used to determine when the cursor changes
            prev_frame_inspect_offset: cursor,
            views,
            focused_view: Some(0),
            ui: crate::ui::Ui::default(),
            selection: None,
            select_begin: None,
            args,
            source,
            col_change_lock_x: false,
            col_change_lock_y: true,
            flash_cursor_timer: Timer::default(),
            stream_end: false,
            just_reloaded: true,
            layout,
            regions: Vec::new(),
            meta_dirty: false,
            stream_read_recv: None,
            cfg,
        };
        this.new_file_readjust(window_height);
        if let Some(offset) = this.args.jump {
            this.center_view_on_offset(offset);
            this.edit_state.cursor = offset;
            this.flash_cursor();
        }
        if let Some(path) = this.meta_path() {
            if path.exists() {
                let data = std::fs::read(path)?;
                let meta = rmp_serde::from_slice(&data)?;
                this.consume_meta(meta);
            }
        }
        Ok(this)
    }
    pub fn reload(&mut self) -> anyhow::Result<()> {
        match &mut self.source {
            Some(Source::File(file)) => {
                self.data = read_contents(&self.args, file)?;
                self.dirty_region = None;
            }
            Some(Source::Stdin(_)) => {
                bail!("Can't reload streaming sources like standard input");
            }
            None => bail!("No file to reload"),
        }
        self.just_reloaded = true;
        Ok(())
    }
    pub fn save(&mut self) -> anyhow::Result<()> {
        let file = match &mut self.source {
            Some(src) => match src {
                Source::File(file) => file,
                Source::Stdin(_) => bail!("Standard input doesn't support saving"),
            },
            None => bail!("No source opened, nothing to save"),
        };
        let offset = self.args.hard_seek.unwrap_or(0);
        file.seek(SeekFrom::Start(offset as u64))?;
        let data_to_write = match self.dirty_region {
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
        self.dirty_region = None;
        Ok(())
    }
    pub fn toggle_debug(&mut self) {
        gamedebug_core::toggle();
    }
    pub fn search_focus(&mut self, offset: usize) {
        self.edit_state.cursor = offset;
        self.center_view_on_offset(offset);
    }

    pub(crate) fn center_view_on_offset(&mut self, offset: usize) {
        if let Some(idx) = self.focused_view {
            self.views[idx].center_on_offset(offset, &self.perspective);
        }
    }

    pub(crate) fn backup_path(&self) -> Option<PathBuf> {
        self.args.file.as_ref().map(|file| {
            let mut os_string = OsString::from(file);
            os_string.push(".hexerator_bak");
            os_string.into()
        })
    }

    pub(crate) fn meta_path(&self) -> Option<PathBuf> {
        self.args.file.as_ref().map(|file| {
            let mut os_string = OsString::from(file);
            os_string.push(".hexerator_meta");
            os_string.into()
        })
    }

    pub(crate) fn widen_dirty_region(&mut self, damage: DamageRegion) {
        match &mut self.dirty_region {
            Some(dirty_region) => {
                if damage.begin() < dirty_region.begin {
                    dirty_region.begin = damage.begin();
                }
                if damage.begin() > dirty_region.end {
                    dirty_region.end = damage.begin();
                }
                let end = damage.end();
                {
                    if end < dirty_region.begin {
                        panic!("Wait, what?");
                    }
                    if end > dirty_region.end {
                        dirty_region.end = end;
                    }
                }
            }
            None => {
                self.dirty_region = Some(Region {
                    begin: damage.begin(),
                    end: damage.end(),
                })
            }
        }
    }

    pub(crate) fn dec_cols(&mut self) {
        self.col_change_impl(|col| *col -= 1);
    }
    fn col_change_impl(&mut self, f: impl FnOnce(&mut usize)) {
        if let Some(idx) = self.focused_view {
            let view = &mut self.views[idx];
            let prev_offset = view.offsets(&self.perspective);
            f(&mut self.perspective.cols);
            self.perspective.clamp_cols();
            view.scroll_to_byte_offset(
                prev_offset.byte,
                &self.perspective,
                self.col_change_lock_x,
                self.col_change_lock_y,
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
        window_height: ViewportScalar,
    ) -> Result<(), anyhow::Error> {
        self.load_file_args(
            Args {
                file: Some(path),
                jump: None,
                hard_seek: None,
                take: None,
                read_only,
                stream: false,
                instance: false,
                load_recent: false,
            },
            window_height,
        )
    }

    /// Readjust to a new file
    fn new_file_readjust(&mut self, window_height: ViewportScalar) {
        self.stream_end = false;
        self.perspective = Perspective {
            region: Region {
                begin: 0,
                end: self.data.len().saturating_sub(1),
            },
            cols: 48,
            flip_row_order: false,
        };
        self.views = default_views(&self.layout, window_height);
    }

    pub fn close_file(&mut self) {
        // We potentially had large data, free it instead of clearing the Vec
        self.data = Vec::new();
        msg_if_fail(self.save_meta(), "Failed to save .hexerator_meta");
        self.args.file = None;
        self.source = None;
    }

    pub(crate) fn restore_backup(&mut self) -> Result<(), anyhow::Error> {
        std::fs::copy(
            &self.backup_path().context("Failed to get backup path")?,
            self.args.file.as_ref().context("No file open")?,
        )?;
        self.reload()
    }

    pub(crate) fn create_backup(&self) -> Result<(), anyhow::Error> {
        std::fs::copy(
            self.args.file.as_ref().context("No file open")?,
            &self.backup_path().context("Failed to get backup path")?,
        )?;
        Ok(())
    }

    pub(crate) fn set_cursor_init(&mut self) {
        self.edit_state.cursor = self.args.jump.unwrap_or(0);
        self.center_view_on_offset(self.edit_state.cursor);
        self.flash_cursor();
    }
    pub fn flash_cursor(&mut self) {
        self.flash_cursor_timer = Timer::set(Duration::from_millis(1500));
    }
    /// If the cursor should be flashing, returns a timer value that can be used to color cursor
    pub fn cursor_flash_timer(&self) -> Option<u32> {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "
        The duration will never be higher than u32 limit.

        It doesn't make sense to set the cursor timer to extremely high values,
        only a few seconds at most.
        "
        )]
        self.flash_cursor_timer
            .overtime()
            .map(|dur| dur.as_millis() as u32)
    }

    pub(crate) fn try_read_stream(&mut self) {
        let Some(idx) = self.focused_view else { return };
        let view = &self.views[idx];
        let view_byte_offset = view.offsets(&self.perspective).byte;
        let bytes_per_page = view.bytes_per_page(&self.perspective);
        // Don't read past what we need for our current view offset
        if view_byte_offset + bytes_per_page < self.data.len() {
            return;
        }
        if self.stream_end {
            return;
        }
        match &self.stream_read_recv {
            Some(recv) => match recv.try_recv() {
                Ok(buf) => {
                    if buf.is_empty() {
                        self.stream_end = true;
                    } else {
                        self.data.extend_from_slice(&buf[..]);
                        self.perspective.region.end = self.data.len() - 1;
                    }
                }
                Err(e) => match e {
                    std::sync::mpsc::TryRecvError::Empty => {}
                    std::sync::mpsc::TryRecvError::Disconnected => self.stream_read_recv = None,
                },
            },
            None => {
                let (tx, rx) = std::sync::mpsc::channel();
                let Some(src) = &mut self.source else { return };
                let mut src_clone = src.clone();
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
    pub fn byte_offset_at_pos(&mut self, x: i16, y: i16) -> Option<(usize, usize)> {
        for (view_idx, view) in self.views.iter().enumerate() {
            if let Some((row, col)) = view.row_col_offset_of_pos(x, y, &self.perspective) {
                return Some((self.perspective.byte_offset_of_row_col(row, col), view_idx));
            }
        }
        None
    }
    pub fn consume_meta(&mut self, meta: Metafile) {
        self.regions = meta.named_regions;
    }
    pub fn make_meta(&self) -> Metafile {
        Metafile {
            named_regions: self.regions.clone(),
        }
    }
    pub fn save_meta(&self) -> anyhow::Result<()> {
        if !self.meta_dirty {
            return Ok(());
        }
        if let Some(path) = self.meta_path() {
            if !path.exists() {
                let ans = rfd::MessageDialog::new()
                    .set_buttons(MessageButtons::YesNo)
                    .set_description(
                        "You have added some meta information. Would you like to save a metafile?",
                    )
                    .show();
                if !ans {
                    return Ok(());
                }
            }
            let meta = self.make_meta();
            let data = rmp_serde::to_vec(&meta)?;
            std::fs::write(path, &data)?;
        }
        Ok(())
    }

    pub(crate) fn load_file_args(
        &mut self,
        mut args: Args,
        window_height: ViewportScalar,
    ) -> anyhow::Result<()> {
        if load_file_from_args(&mut args, &mut self.cfg, &mut self.source, &mut self.data) {
            self.args = args;
        }
        self.new_file_readjust(window_height);
        Ok(())
    }
}

fn default_views(layout: &Layout, window_height: ViewportScalar) -> Vec<View> {
    vec![
        View {
            viewport_rect: ViewportRect {
                x: 0,
                y: layout.top_gap,
                w: 960,
                h: window_height - layout.bottom_gap,
            },
            kind: ViewKind::Hex,
            col_w: layout.font_size * 2,
            row_h: layout.font_size,
            scroll_offset: ScrollOffset::default(),
            scroll_speed: 1,
        },
        View {
            viewport_rect: ViewportRect {
                x: 962,
                y: layout.top_gap,
                w: 480,
                h: window_height - layout.bottom_gap,
            },
            kind: ViewKind::Ascii,
            col_w: layout.font_size,
            row_h: layout.font_size,
            scroll_offset: ScrollOffset::default(),
            scroll_speed: 1,
        },
        View {
            viewport_rect: ViewportRect {
                x: 1444,
                y: layout.top_gap,
                w: 200,
                h: window_height - layout.bottom_gap,
            },
            kind: ViewKind::Block,
            col_w: 4,
            row_h: 4,
            scroll_offset: ScrollOffset::default(),
            scroll_speed: 1,
        },
    ]
}

/// Returns if the file was actually loaded.
fn load_file_from_args(
    args: &mut Args,
    cfg: &mut Config,
    source: &mut Option<Source>,
    data: &mut Vec<u8>,
) -> bool {
    if let Some(file_arg) = &args.file {
        if file_arg.as_os_str() == "-" {
            *source = Some(Source::Stdin(std::io::stdin()));
            args.stream = true;
            true
        } else {
            let result: Result<(), anyhow::Error> = try {
                let mut file = open_file(file_arg, args.read_only)?;
                data.clear();
                if let Some(path) = &mut args.file {
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
                cfg.recent.use_(args.clone());
                if !args.stream {
                    *data = read_contents(&*args, &mut file)?;
                }
                *source = Some(Source::File(file));
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

fn read_contents(args: &Args, file: &mut File) -> anyhow::Result<Vec<u8>> {
    let seek = args.hard_seek.unwrap_or(0);
    file.seek(SeekFrom::Start(seek as u64))?;
    let mut data = Vec::new();
    match args.take {
        Some(amount) => (&*file).take(amount as u64).read_to_end(&mut data)?,
        None => file.read_to_end(&mut data)?,
    };
    Ok(data)
}
