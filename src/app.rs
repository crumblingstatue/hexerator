use std::ffi::OsString;

use egui_inspect::{derive::Inspect, UiExt};
use egui_sfml::egui::{self, Ui};
use sfml::graphics::Vertex;

use crate::{input::Input, views::ColorMethod, EditTarget, FindDialog, InteractMode, Region};

/// The hexerator application state
#[derive(Inspect, Debug)]
pub struct App {
    /// Font size
    pub font_size: u32,
    /// Block size for block view
    pub block_size: u8,
    /// The default view
    pub view: View,
    // Maximum number of visible hex columns that can be shown on screen.
    // ascii is double this amount.
    pub max_visible_cols: usize,
    /// Path to the file we're editing
    pub path: OsString,
    pub dirty: bool,
    pub data: Vec<u8>,
    pub show_debug_panel: bool,
    pub col_width: u8,
    // The editing byte offset
    pub cursor: usize,
    #[inspect_with(inspect_vertices)]
    pub vertices: Vec<Vertex>,
    pub input: Input,
    pub interact_mode: InteractMode,
    pub top_gap: i64,
    pub view_x: i64,
    pub view_y: i64,
    // The amount scrolled per frame in view mode
    pub scroll_speed: i64,
    pub color_method: ColorMethod,
    // The value of the cursor on the previous frame. Used to determine when the cursor changes
    pub cursor_prev_frame: usize,
    pub edit_target: EditTarget,
    pub row_height: u8,
    pub show_hex: bool,
    pub show_text: bool,
    pub show_block: bool,
    // The half digit when the user begins to type into a hex view
    pub hex_edit_half_digit: Option<u8>,
    pub u8_buf: String,
    pub find_dialog: FindDialog,
    pub selection: Option<Region>,
    pub select_begin: Option<usize>,
    pub fill_text: String,
    pub backup_path: OsString,
}

fn inspect_vertices(vertices: &mut Vec<Vertex>, ui: &mut Ui, mut id_source: u64) {
    ui.inspect_iter_with_mut(
        &format!("Vec<Vertex> [{}]", vertices.len()),
        vertices,
        &mut id_source,
        |ui, i, vert, id_source| {
            ui.horizontal(|ui| {
                ui.label(i.to_string());
                ui.property("x", &mut vert.position.x, id_source);
                ui.property("y", &mut vert.position.y, id_source);
            });
        },
    );
}

/// A view into the data
#[derive(Inspect, Debug)]
pub struct View {
    /// The starting offset where the view starts from
    pub start_offset: usize,
    /// How many rows the view displays (how tall it is)
    pub rows: usize,
    /// How many columns the view displays (how wide it is)
    pub cols: usize,
}

pub enum CursorViewStatus {
    Inside,
    Before,
    After,
}

impl App {
    pub fn new(path: OsString) -> Self {
        let data = std::fs::read(&path).unwrap();
        let top_gap = 30;
        let cursor = 0;
        Self {
            font_size: 14,
            block_size: 4,
            view: View {
                start_offset: 0,
                rows: 67,
                cols: 48,
            },
            max_visible_cols: 75,
            path: path.clone(),
            dirty: false,
            data,
            show_debug_panel: false,
            col_width: 26,
            cursor,
            vertices: Vec::new(),
            input: Input::default(),
            interact_mode: InteractMode::View,
            // The top part where the top panel is. You should try to position stuff so it's not overdrawn
            // by the top panel
            top_gap,
            // The x pixel offset of the scrollable view
            view_x: 0,
            // The y pixel offset of the scrollable view
            view_y: -top_gap,
            // The amount scrolled per frame in view mode
            scroll_speed: 4,
            color_method: ColorMethod::Default,
            // The value of the cursor on the previous frame. Used to determine when the cursor changes
            cursor_prev_frame: cursor,
            edit_target: EditTarget::Hex,
            row_height: 16,
            show_hex: true,
            show_text: true,
            show_block: false,
            // The half digit when the user begins to type into a hex view
            hex_edit_half_digit: None,
            u8_buf: String::new(),
            find_dialog: FindDialog::default(),
            selection: None,
            select_begin: None,
            fill_text: String::new(),
            backup_path: {
                let mut new = path;
                new.push(".hexerator_bak");
                new
            },
        }
    }
    pub fn reload(&mut self) {
        self.data = std::fs::read(&self.path).unwrap();
        self.dirty = false;
    }
    pub fn save(&mut self) {
        std::fs::write(&self.path, &self.data).unwrap();
        self.dirty = false;
    }
    pub fn toggle_debug(&mut self) {
        self.show_debug_panel ^= true;
        gamedebug_core::toggle();
    }
    pub fn ascii_display_x_offset(&self) -> i64 {
        self.view.cols as i64 * i64::from(self.col_width) + 12
    }
    pub fn cursor_view_status(cursor: usize, view: &View) -> CursorViewStatus {
        if cursor < view.start_offset {
            CursorViewStatus::Before
        } else if cursor > view.start_offset + view.rows * view.cols {
            CursorViewStatus::After
        } else {
            CursorViewStatus::Inside
        }
    }
    pub fn search_focus(cursor: &mut usize, view: &mut View, off: usize) {
        // Focus the search result in the hex view
        *cursor = off;
        match Self::cursor_view_status(*cursor, view) {
            CursorViewStatus::Before => {
                view.start_offset = off.saturating_sub((view.rows - 1) * (view.cols - 1))
            }
            CursorViewStatus::After => view.start_offset = off - (view.rows + view.cols),
            CursorViewStatus::Inside => {}
        }
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
}
