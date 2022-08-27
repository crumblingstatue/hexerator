mod edit_state;
pub mod interact_mode;
pub mod perspective;
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
use rfd::MessageButtons;
use serde::{Deserialize, Serialize};
use slotmap::{new_key_type, SlotMap};

use crate::{
    args::{Args, SourceArgs},
    config::Config,
    damage_region::DamageRegion,
    input::Input,
    metafile::Metafile,
    region::Region,
    shell::{msg_if_fail, msg_warn},
    source::{Source, SourceAttributes, SourcePermissions, SourceProvider, SourceState},
    timer::Timer,
    view::{HexData, TextData, View, ViewKind, ViewportRect, ViewportScalar},
};

use self::{edit_state::EditState, interact_mode::InteractMode, perspective::Perspective};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NamedView {
    pub name: String,
    pub view: View,
}

/// An event that can be triggered weakly.
///
/// A weak trigger can be called repeatedly every frame, and it will only
/// trigger on the first time.
#[derive(Default)]
pub enum EventTrigger {
    /// Initial state
    #[default]
    Init,
    /// The event was triggered
    Triggered,
    /// The event was already triggered once
    Inactive,
}

impl EventTrigger {
    pub fn weak_trigger(&mut self) {
        match self {
            Self::Init => *self = Self::Triggered,
            Self::Triggered => *self = Self::Inactive,
            Self::Inactive => {}
        }
    }
    pub fn triggered(&self) -> bool {
        matches!(self, Self::Triggered)
    }
    pub fn reset(&mut self) {
        *self = Self::Init;
    }
}

new_key_type! {
    pub struct PerspectiveKey;
    pub struct RegionKey;
}

pub type PerspectiveMap = SlotMap<PerspectiveKey, Perspective>;
pub type RegionMap = SlotMap<RegionKey, NamedRegion>;

/// The hexerator application state
pub struct App {
    /// The default perspective
    pub perspectives: PerspectiveMap,
    pub dirty_region: Option<Region>,
    pub data: Vec<u8>,
    pub edit_state: EditState,
    pub input: Input,
    pub interact_mode: InteractMode,
    pub named_views: Vec<NamedView>,
    pub focused_view: Option<usize>,
    /// The rectangle area that's available for the hex interface
    pub hex_iface_rect: ViewportRect,
    pub ui: crate::ui::Ui,
    pub resize_views: EventTrigger,
    /// Automatic view layout every frame
    pub auto_view_layout: bool,
    /// "a" point of selection. Could be smaller or larger than "b".
    /// The length of selection is absolute difference between a and b
    pub select_a: Option<usize>,
    /// "b" point of selection. Could be smaller or larger than "a".
    /// The length of selection is absolute difference between a and b
    pub select_b: Option<usize>,
    pub args: Args,
    pub source: Option<Source>,
    pub col_change_lock_x: bool,
    pub col_change_lock_y: bool,
    flash_cursor_timer: Timer,
    pub just_reloaded: bool,
    pub regions: SlotMap<RegionKey, NamedRegion>,
    /// Whether metafile needs saving
    pub meta_dirty: bool,
    pub stream_read_recv: Option<Receiver<Vec<u8>>>,
    pub cfg: Config,
    /// Whether to scissor views when drawing them. Useful to disable when debugging rendering.
    pub scissor_views: bool,
    /// If true, auto-reload the current file at specified interval
    pub auto_reload: bool,
    /// Auto-reload interval in milliseconds
    pub auto_reload_interval_ms: u32,
    last_reload: Instant,
    pub preferences: Preferences,
    pub bg_color: [f32; 3],
    /// When alt is being held, it shows things like names of views as overlays
    pub show_alt_overlay: bool,
}

#[derive(Debug, Default)]
pub struct Preferences {
    /// Move the edit cursor with the cursor keys, instead of block cursor
    pub move_edit_cursor: bool,
    /// Immediately apply changes when editing a value, instead of having
    /// to type everything or press enter
    pub quick_edit: bool,
    /// Don't move the cursor after editing is finished
    pub sticky_edit: bool,
    /// Automatically save when editing is finished
    pub auto_save: bool,
    /// Keep metadata when loading.
    pub keep_meta: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NamedRegion {
    pub name: String,
    pub region: Region,
}

impl App {
    pub fn new(mut args: Args, mut cfg: Config, font: &Font) -> anyhow::Result<Self> {
        let mut data = Vec::new();
        let mut source = None;
        if args.load_recent && let Some(recent) = cfg.recent.most_recent() {
            args = recent.clone();
        }
        let load_success = load_file_from_args(&mut args, &mut cfg, &mut source, &mut data);
        let mut this = Self {
            scissor_views: true,
            perspectives: SlotMap::default(),
            dirty_region: None,
            data,
            edit_state: EditState::default(),
            input: Input::default(),
            interact_mode: InteractMode::View,
            named_views: Vec::default(),
            focused_view: None,
            ui: crate::ui::Ui::default(),
            resize_views: EventTrigger::default(),
            auto_view_layout: true,
            select_a: None,
            select_b: None,
            args,
            source,
            col_change_lock_x: false,
            col_change_lock_y: true,
            flash_cursor_timer: Timer::default(),
            just_reloaded: true,
            regions: SlotMap::default(),
            meta_dirty: false,
            stream_read_recv: None,
            cfg,
            auto_reload: false,
            auto_reload_interval_ms: 250,
            last_reload: Instant::now(),
            preferences: Preferences::default(),
            hex_iface_rect: ViewportRect::default(),
            bg_color: [0.; 3],
            show_alt_overlay: false,
        };
        if load_success {
            this.new_file_readjust(font, &ViewportRect::default());
            try_consume_metafile(&mut this)?;
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
                    self.data = read_contents(&self.args, file)?;
                    self.dirty_region = None;
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
            self.named_views[idx]
                .view
                .center_on_offset(offset, &self.perspectives);
        }
    }

    pub(crate) fn backup_path(&self) -> Option<PathBuf> {
        self.args.src.file.as_ref().map(|file| {
            let mut os_string = OsString::from(file);
            os_string.push(".hexerator_bak");
            os_string.into()
        })
    }

    pub(crate) fn meta_path(&self) -> Option<PathBuf> {
        self.args.src.file.as_ref().map(|file| {
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
            let view = &mut self.named_views[idx].view;
            col_change_impl_view_perspective(
                view,
                &mut self.perspectives,
                &self.regions,
                f,
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
                instance: false,
                load_recent: false,
            },
            font,
        )
    }

    /// Readjust to a new file
    fn new_file_readjust(&mut self, font: &Font, hex_iface_rect: &ViewportRect) {
        let def_region = self.regions.insert(NamedRegion {
            name: "Default region".into(),
            region: Region {
                begin: 0,
                end: self.data.len().saturating_sub(1),
            },
        });
        let default_perspective = self.perspectives.insert(Perspective {
            region: def_region,
            cols: 48,
            flip_row_order: false,
        });
        self.named_views = default_views(font, hex_iface_rect, default_perspective);
        // If we have no focused view, let's focus on the default view
        if self.focused_view.is_none() {
            self.focused_view = Some(0);
        }
    }

    pub fn close_file(&mut self) {
        // We potentially had large data, free it instead of clearing the Vec
        self.data = Vec::new();
        msg_if_fail(self.save_meta(), "Failed to save .hexerator_meta");
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
        let Some(src) = &mut self.source else { return };
        if !src.attr.stream {
            return;
        };
        let Some(idx) = self.focused_view else { return };
        let view = &self.named_views[idx].view;
        let view_byte_offset = view.offsets(&self.perspectives, &self.regions).byte;
        let bytes_per_page = view.bytes_per_page(&self.perspectives);
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
                        let perspective = &self.perspectives[view.perspective];
                        let region = &mut self.regions[perspective.region].region;
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
    pub fn byte_offset_at_pos(&mut self, x: i16, y: i16) -> Option<(usize, usize)> {
        for (view_idx, view) in self.named_views.iter().map(|v| &v.view).enumerate() {
            if !view.active {
                continue;
            }
            if let Some((row, col)) =
                view.row_col_offset_of_pos(x, y, &self.perspectives, &self.regions)
            {
                return Some((
                    self.perspectives[view.perspective].byte_offset_of_row_col(
                        row,
                        col,
                        &self.regions,
                    ),
                    view_idx,
                ));
            }
        }
        None
    }
    pub fn view_idx_at_pos(&self, x: i16, y: i16) -> Option<usize> {
        for (view_idx, view) in self.named_views.iter().enumerate() {
            if !view.view.active {
                continue;
            }
            if view.view.viewport_rect.contains_pos(x, y) {
                return Some(view_idx);
            }
        }
        None
    }
    pub fn consume_meta(&mut self, meta: Metafile) {
        self.regions = meta.named_regions;
        self.perspectives = meta.perspectives;
        self.named_views = meta.views;
        for view in &mut self.named_views {
            // Needed to initialize edit buffers, etc.
            view.view.adjust_state_to_kind();
        }
    }
    pub fn make_meta(&self) -> Metafile {
        Metafile {
            named_regions: self.regions.clone(),
            perspectives: self.perspectives.clone(),
            views: self.named_views.clone(),
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
            self.save_meta_to_file(path)?;
        }
        Ok(())
    }

    pub fn save_meta_to_file(&self, path: PathBuf) -> Result<(), anyhow::Error> {
        let meta = self.make_meta();
        let data = rmp_serde::to_vec(&meta)?;
        std::fs::write(path, &data)?;
        Ok(())
    }

    pub(crate) fn load_file_args(&mut self, mut args: Args, font: &Font) -> anyhow::Result<()> {
        if load_file_from_args(&mut args, &mut self.cfg, &mut self.source, &mut self.data) {
            self.args = args;
        }
        if !self.preferences.keep_meta {
            let iface_rect = self.hex_iface_rect;
            self.new_file_readjust(font, &iface_rect);
            try_consume_metafile(self)?;
        }
        Ok(())
    }
    /// Called every frame
    pub(crate) fn update(&mut self) {
        if self.auto_view_layout || self.resize_views.triggered() {
            named_views_auto_layout(&mut self.named_views, &self.hex_iface_rect);
        }
        if self.auto_reload
            && self.last_reload.elapsed().as_millis() >= u128::from(self.auto_reload_interval_ms)
        {
            if msg_if_fail(self.reload(), "Auto-reload fail").is_some() {
                self.auto_reload = false;
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
}

fn try_consume_metafile(this: &mut App) -> Result<(), anyhow::Error> {
    if let Some(path) = this.meta_path() {
        if path.exists() {
            consume_meta_from_file(path, this)?;
        }
    };
    Ok(())
}

pub fn consume_meta_from_file(path: PathBuf, this: &mut App) -> Result<(), anyhow::Error> {
    let data = std::fs::read(path)?;
    let meta = rmp_serde::from_slice(&data)?;
    this.consume_meta(meta);
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
    view.scroll_to_byte_offset(prev_offset.byte, perspectives, lock_x, lock_y);
}

fn named_views_auto_layout(named_views: &mut [NamedView], hex_iface_rect: &ViewportRect) {
    if hex_iface_rect.w == 0 {
        // Can't deal with 0 viewport w, do nothing
        return;
    }
    // Horizontal auto layout algorithm by Callie
    let padding = 4;
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        reason = "Number of views won't exceed i16"
    )]
    let n_views = named_views.iter().filter(|v| v.view.active).count() as ViewportScalar;
    if n_views == 0 {
        return;
    }
    #[expect(clippy::cast_sign_loss, reason = "n_views is always positive")]
    let slice = hex_iface_rect.w / (2i16.pow(n_views as u32) - 1);
    let mut x = hex_iface_rect.x + hex_iface_rect.w;
    for (i, rect) in named_views
        .iter_mut()
        .rev()
        .filter_map(|v| v.view.active.then_some(&mut v.view.viewport_rect))
        .enumerate()
    {
        // Horizontal layout
        #[expect(
            clippy::cast_possible_truncation,
            reason = "Number of views doesn't exceed u32"
        )]
        {
            rect.w = slice * 2i16.pow(i as u32) - padding;
        }
        x -= rect.w + padding;
        rect.x = x;
        // Vertical is always the same (for now)
        rect.y = hex_iface_rect.y;
        rect.h = hex_iface_rect.h;
    }
}

pub fn default_views(
    font: &Font,
    hex_iface_rect: &ViewportRect,
    perspective: PerspectiveKey,
) -> Vec<NamedView> {
    let mut v = vec![
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
    ];
    named_views_auto_layout(&mut v, hex_iface_rect);
    v
}

/// Returns if the file was actually loaded.
fn load_file_from_args(
    args: &mut Args,
    cfg: &mut Config,
    source: &mut Option<Source>,
    data: &mut Vec<u8>,
) -> bool {
    if let Some(file_arg) = &args.src.file {
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
                let mut file = open_file(file_arg, args.src.read_only)?;
                data.clear();
                if let Some(path) = &mut args.src.file {
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
                if !args.src.stream {
                    *data = read_contents(&*args, &mut file)?;
                }
                *source = Some(Source {
                    provider: SourceProvider::File(file),
                    attr: SourceAttributes {
                        seekable: true,
                        stream: args.src.stream,
                        permissions: SourcePermissions {
                            read: true,
                            write: !args.src.read_only,
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

fn read_contents(args: &Args, file: &mut File) -> anyhow::Result<Vec<u8>> {
    let seek = args.src.hard_seek.unwrap_or(0);
    file.seek(SeekFrom::Start(seek as u64))?;
    let mut data = Vec::new();
    match args.src.take {
        Some(amount) => (&*file).take(amount as u64).read_to_end(&mut data)?,
        None => file.read_to_end(&mut data)?,
    };
    Ok(data)
}
