mod edit_state;
pub mod edit_target;
pub mod interact_mode;
mod layout;
pub mod presentation;
mod view;

use std::{
    ffi::OsString,
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::{bail, Context};
use rfd::MessageButtons;
use serde::{Deserialize, Serialize};

use crate::{
    args::Args, damage_region::DamageRegion, input::Input, metafile::Metafile, msg_if_fail,
    region::Region, source::Source, timer::Timer,
};

use self::{
    edit_state::EditState, edit_target::EditTarget, interact_mode::InteractMode, layout::Layout,
    presentation::Presentation, view::View,
};

/// The hexerator application state
#[derive(Debug)]
pub struct App {
    /// The default view
    pub view: View,
    pub dirty_region: Option<Region>,
    pub data: Vec<u8>,
    pub edit_state: EditState,
    pub input: Input,
    pub interact_mode: InteractMode,
    pub view_x: i64,
    pub view_y: i64,
    // The amount scrolled per frame in view mode
    pub scroll_speed: i64,
    pub presentation: Presentation,
    // The value of the cursor on the previous frame. Used to determine when the cursor changes
    pub prev_frame_inspect_offset: usize,
    pub edit_target: EditTarget,
    pub show_hex: bool,
    pub show_text: bool,
    pub show_block: bool,
    pub ui: crate::ui::Ui,
    pub selection: Option<Region>,
    pub select_begin: Option<usize>,
    pub args: Args,
    pub source: Option<Source>,
    pub col_change_lock_x: bool,
    pub col_change_lock_y: bool,
    flash_cursor_timer: Timer,
    stream_end: bool,
    pub just_reloaded: bool,
    pub layout: Layout,
    pub regions: Vec<NamedRegion>,
    /// Whether metafile needs saving
    pub meta_dirty: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NamedRegion {
    pub name: String,
    pub region: Region,
}

impl App {
    pub fn new(mut args: Args, window_height: u32) -> anyhow::Result<Self> {
        let data;
        let source;
        match &args.file {
            Some(file_arg) => {
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
        let mut this = Self {
            view: View {
                start_offset: 0,
                rows: 67,
                cols: 48,
            },
            dirty_region: None,
            data,
            edit_state: EditState::default(),
            input: Input::default(),
            interact_mode: InteractMode::View,
            // The top part where the top panel is. You should try to position stuff so it's not overdrawn
            // by the top panel
            // The x pixel offset of the scrollable view
            view_x: 0,
            // The y pixel offset of the scrollable view
            view_y: -layout.top_gap,
            // The amount scrolled per frame in view mode
            scroll_speed: 4,
            presentation: Presentation::default(),
            // The value of the cursor on the previous frame. Used to determine when the cursor changes
            prev_frame_inspect_offset: cursor,
            edit_target: EditTarget::Hex,
            show_hex: true,
            show_text: true,
            show_block: false,
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
        self.ui.show_debug_panel ^= true;
        gamedebug_core::toggle();
    }
    pub fn ascii_display_x_offset(&self) -> i64 {
        self.view.cols as i64 * i64::from(self.layout.col_width) + 12
    }
    pub fn search_focus(&mut self, offset: usize) {
        self.edit_state.cursor = offset;
        self.center_view_on_offset(offset);
    }

    pub(crate) fn block_display_x_offset(&self) -> i64 {
        self.ascii_display_x_offset() * 2
    }

    pub(crate) fn clamp_view(&mut self) {
        if self.view_x < -100 {
            self.view_x = -100;
        }
        if self.view_y < -100 {
            self.view_y = -100;
        }
    }

    pub(crate) fn center_view_on_offset(&mut self, offset: usize) {
        let (row, col) = self.view.offset_row_col(offset);
        self.view_x = (col as i64 * self.layout.col_width as i64) - 200;
        self.view_y = (row as i64 * self.layout.row_height as i64) - 200;
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
        let prev_offset = self.view_offsets();
        self.view.cols -= 1;
        self.clamp_cols();
        self.set_view_to_byte_offset(prev_offset.byte);
    }
    pub(crate) fn inc_cols(&mut self) {
        let prev_offset = self.view_offsets();
        self.view.cols += 1;
        self.clamp_cols();
        self.set_view_to_byte_offset(prev_offset.byte);
    }
    pub(crate) fn halve_cols(&mut self) {
        let prev_offset = self.view_offsets();
        self.view.cols /= 2;
        self.clamp_cols();
        self.set_view_to_byte_offset(prev_offset.byte);
    }
    pub(crate) fn double_cols(&mut self) {
        let prev_offset = self.view_offsets();
        self.view.cols *= 2;
        self.clamp_cols();
        self.set_view_to_byte_offset(prev_offset.byte);
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
        self.view.cols = self.view.cols.clamp(1, self.data.len());
    }
    /// Calculate the (row, col, byte) offset where the view starts showing from
    pub fn view_offsets(&self) -> ViewOffsets {
        let view_y = self.view_y + self.layout.top_gap;
        let row_offset: usize = (view_y / self.layout.row_height as i64)
            .try_into()
            .unwrap_or(0);
        let col_offset: usize = (self.view_x / self.layout.col_width as i64)
            .try_into()
            .unwrap_or(0);
        ViewOffsets {
            row: row_offset,
            col: col_offset,
            byte: row_offset * self.view.cols + col_offset,
        }
    }

    pub fn set_view_to_byte_offset(&mut self, offset: usize) {
        let (row, col) = self.view.offset_row_col(offset);
        if self.col_change_lock_x {
            self.view_x = (col * self.layout.col_width as usize) as i64;
        }
        if self.col_change_lock_y {
            self.view_y = ((row * self.layout.row_height as usize) as i64) - self.layout.top_gap;
        }
    }

    pub(crate) fn load_file(&mut self, path: PathBuf) -> Result<(), anyhow::Error> {
        let mut file = open_file(&path, self.args.read_only)?;
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

    pub(crate) fn data_height(&self) -> i64 {
        let len = self.data.len();
        let rows = len as i64 / self.view.cols as i64;
        rows * self.layout.row_height as i64
    }

    pub(crate) fn view_area(&self) -> i64 {
        self.layout.window_height as i64 - self.layout.top_gap - self.layout.bottom_gap
    }

    pub(crate) fn try_read_stream(&mut self) {
        let view_byte_offset = self.view_offsets().byte;
        let bytes_per_page = self.view.rows * self.view.cols;
        // Don't read past what we need for our current view offset
        if view_byte_offset + bytes_per_page < self.data.len() {
            return;
        }
        if self.stream_end {
            return;
        }
        let Some(src) = &mut self.source else { return };
        let buffer_size = 1024;
        let mut buf = vec![0; buffer_size];
        let amount = src.read(&mut buf).unwrap();
        if amount == 0 {
            self.stream_end = true;
        } else {
            self.data.extend_from_slice(&buf[..amount]);
        }
    }
    // Byte offset of a pixel position in the view
    pub fn pixel_pos_byte_offset(&mut self, x: i32, y: i32) -> usize {
        let x: i64 = self.view_x + i64::from(x);
        let y: i64 = self.view_y + i64::from(y);
        let ascii_display_x_offset = self.ascii_display_x_offset();
        let col_y = y / i64::from(self.layout.row_height);
        let col_x = if x < ascii_display_x_offset {
            x / i64::from(self.layout.col_width)
        } else {
            let x_rel = x - ascii_display_x_offset;
            x_rel / i64::from(self.layout.col_width / 2)
        };
        (usize::try_from(col_y).unwrap_or(0) * self.view.cols + usize::try_from(col_x).unwrap_or(0))
            + self.view.start_offset
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
impl View {
    /// Calculate the row and column for a given offset when viewed through this View
    fn offset_row_col(&self, offset: usize) -> (usize, usize) {
        (offset / self.cols, offset % self.cols)
    }
}
