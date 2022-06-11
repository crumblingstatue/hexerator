use std::path::PathBuf;

use egui_inspect::{derive::Inspect, UiExt};
use egui_sfml::egui::{self, Ui};
use sfml::graphics::Vertex;

use crate::{
    args::Args, color::ColorMethod, input::Input, EditTarget, FindDialog, InteractMode, Region,
};

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
    pub invert_color: bool,
    pub bg_color: [f32; 3],
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
    pub center_offset_input: String,
    #[opaque]
    pub args: Args,
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

impl App {
    pub fn new(args: Args) -> Self {
        let data = std::fs::read(&args.file).unwrap();
        let top_gap = 30;
        let cursor = 0;
        let mut this = Self {
            font_size: 14,
            block_size: 4,
            view: View {
                start_offset: 0,
                rows: 67,
                cols: 48,
            },
            max_visible_cols: 75,
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
            invert_color: false,
            bg_color: [0.; 3],
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
            center_offset_input: String::new(),
            args,
        };
        if let Some(offset) = this.args.jump {
            this.center_view_on_offset(offset);
            this.cursor = offset;
        }
        this
    }
    pub fn reload(&mut self) {
        self.data = std::fs::read(&self.args.file).unwrap();
        self.dirty = false;
    }
    pub fn save(&mut self) {
        std::fs::write(&self.args.file, &self.data).unwrap();
        self.dirty = false;
    }
    pub fn toggle_debug(&mut self) {
        self.show_debug_panel ^= true;
        gamedebug_core::toggle();
    }
    pub fn ascii_display_x_offset(&self) -> i64 {
        self.view.cols as i64 * i64::from(self.col_width) + 12
    }
    pub fn search_focus(&mut self, offset: usize) {
        self.cursor = offset;
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
        self.view_x = (col as i64 * self.col_width as i64) - 200;
        self.view_y = (row as i64 * self.row_height as i64) - 200;
    }

    pub(crate) fn backup_path(&self) -> PathBuf {
        self.args.file.join(".hexerator_bak")
    }
}
impl View {
    /// Calculate the row and column for a given offset when viewed through this View
    fn offset_row_col(&self, offset: usize) -> (usize, usize) {
        (offset / self.cols, offset % self.cols)
    }
}
