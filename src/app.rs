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
    msg_if_fail,
    region::Region,
    source::Source,
    timer::Timer,
    view::{ScrollOffset, View, ViewKind, ViewportRect},
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
    pub fn new(mut args: Args, window_height: u32, mut cfg: Config) -> anyhow::Result<Self> {
        let data;
        let source;
        if args.load_recent && let Some(recent) = cfg.recent.most_recent() {
            args.file = Some(recent.clone());
        }
        match &args.file {
            Some(file_arg) => {
                cfg.recent.use_(file_arg.clone());
                if file_arg.as_os_str() == "-" {
                    source = Some(Source::Stdin(std::io::stdin()));
                    data = Vec::new();
                    args.stream = true;
                } else {
                    let mut file = open_file(file_arg, args.read_only)?;
                    if !args.stream {
                        data = read_contents(&args, &mut file)?;
                    } else {
                        data = Vec::new();
                    }
                    source = Some(Source::File(file));
                }
            }
            None => {
                data = Vec::new();
                source = None;
            }
        }
        let layout = Layout::new(window_height);
        let cursor = 0;
        let mut views = vec![
            View {
                viewport_rect: ViewportRect {
                    x: 0,
                    y: layout.top_gap,
                    w: 960,
                    h: window_height as i16 - layout.bottom_gap,
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
                    h: window_height as i16 - layout.bottom_gap,
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
                    h: window_height as i16 - layout.bottom_gap,
                },
                kind: ViewKind::Block,
                col_w: 4,
                row_h: 4,
                scroll_offset: ScrollOffset::default(),
                scroll_speed: 1,
            },
        ];
        views[0].go_home();
        let mut this = Self {
            scissor_views: true,
            perspective: Perspective {
                region: Region {
                    begin: 0,
                    end: data.len(),
                },
                cols: 48,
            },
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
        file.seek(SeekFrom::Start(offset))?;
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

    pub(crate) fn center_view_on_offset(&mut self, _offset: usize) {
        todo!()
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
        //let prev_offset = self.view_offsets();
        self.perspective.cols -= 1;
        self.clamp_cols();
        //self.set_view_to_byte_offset(prev_offset.byte);
    }
    pub(crate) fn inc_cols(&mut self) {
        //let prev_offset = self.view_offsets();
        self.perspective.cols += 1;
        self.clamp_cols();
        //self.set_view_to_byte_offset(prev_offset.byte);
    }
    pub(crate) fn halve_cols(&mut self) {
        //let prev_offset = self.view_offsets();
        self.perspective.cols /= 2;
        self.clamp_cols();
        //self.set_view_to_byte_offset(prev_offset.byte);
    }
    pub(crate) fn double_cols(&mut self) {
        //let prev_offset = self.view_offsets();
        self.perspective.cols *= 2;
        self.clamp_cols();
        //self.set_view_to_byte_offset(prev_offset.byte);
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
    fn clamp_cols(&mut self) {
        self.perspective.cols = self.perspective.cols.clamp(1, self.data.len());
    }
    /// Calculate the (row, col, byte) offset where the view starts showing from
    pub fn view_offsets(&self) -> ViewOffsets {
        todo!()
    }

    pub fn set_view_to_byte_offset(&mut self, _offset: usize) {
        todo!()
    }

    pub(crate) fn load_file(
        &mut self,
        path: PathBuf,
        read_only: bool,
    ) -> Result<(), anyhow::Error> {
        self.cfg.recent.use_(path.clone());
        let mut file = open_file(&path, read_only)?;
        self.data = read_contents(&self.args, &mut file)?;
        self.source = Some(Source::File(file));
        self.args.file = Some(path);
        Ok(())
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
        self.flash_cursor_timer
            .overtime()
            .map(|dur| dur.as_millis() as u32)
    }

    pub(crate) fn try_read_stream(&mut self) {
        let view_byte_offset = self.view_offsets().byte;
        // TODO: Implement properly
        // It should probably take the most far-reaching view and use the
        // bytes_per_page value of that.
        let bytes_per_page = 100;
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
                    let amount = src_clone.read(&mut buf).unwrap();
                    buf.truncate(amount);
                    tx.send(buf).unwrap();
                });
            }
        }
    }
    // Byte offset of a pixel position in the view
    pub fn pixel_pos_byte_offset(&mut self, _x: i32, _y: i32) -> usize {
        // TODO: Implement
        0
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
}

pub struct ViewOffsets {
    pub row: usize,
    pub col: usize,
    pub byte: usize,
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
    file.seek(SeekFrom::Start(seek))?;
    let mut data = Vec::new();
    match args.take {
        Some(amount) => (&*file).take(amount).read_to_end(&mut data)?,
        None => file.read_to_end(&mut data)?,
    };
    Ok(data)
}
impl Perspective {
    /// Calculate the row and column for a given offset when viewed through this View
    fn offset_row_col(&self, offset: usize) -> (usize, usize) {
        (offset / self.cols, offset % self.cols)
    }
}
