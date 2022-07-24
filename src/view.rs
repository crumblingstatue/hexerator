use gamedebug_core::{imm_msg, per_msg};

use crate::app::perspective::Perspective;

mod draw;

/// A rectangular view in the viewport looking through a perspective at the data with a flavor
/// of rendering/interaction (hex/ascii/block/etc.)
///
/// There can be different views through the same perspective.
/// By default they sync their offsets, but each view can show different amounts of data
/// depending on block size of its items, and its relative size in the viewport.
#[derive(Debug)]
pub struct View {
    /// The rectangle to occupy in the viewport
    pub viewport_rect: ViewportRect,
    /// The kind of view (hex, ascii, block, etc)
    pub kind: ViewKind,
    /// Width of a column
    pub col_w: u8,
    /// Height of a row
    pub row_h: u8,
    /// The scrolling offset
    pub scroll_offset: ScrollOffset,
    /// The amount scrolled for a single scroll operation, in pixels
    pub scroll_speed: i16,
}

impl View {
    pub fn scroll_x(&mut self, amount: i16) {
        scroll_impl(
            &mut self.scroll_offset.col,
            &mut self.scroll_offset.pix_xoff,
            self.col_w.into(),
            amount,
        )
    }
    pub fn scroll_y(&mut self, amount: i16) {
        scroll_impl(
            &mut self.scroll_offset.row,
            &mut self.scroll_offset.pix_yoff,
            self.row_h.into(),
            amount,
        )
    }

    pub(crate) fn sync_to(
        &mut self,
        src_row: usize,
        src_yoff: i16,
        src_col: usize,
        src_xoff: i16,
        src_row_h: u8,
        src_col_w: u8,
    ) {
        self.scroll_offset.row = src_row;
        self.scroll_offset.col = src_col;
        let y_ratio = src_row_h as f64 / self.row_h as f64;
        let x_ratio = src_col_w as f64 / self.col_w as f64;
        self.scroll_offset.pix_yoff = (src_yoff as f64 / y_ratio) as i16;
        self.scroll_offset.pix_xoff = (src_xoff as f64 / x_ratio) as i16;
    }

    pub(crate) fn scroll_page_down(&mut self) {
        self.scroll_y(self.viewport_rect.h);
    }

    pub(crate) fn scroll_page_up(&mut self) {
        self.scroll_y(-self.viewport_rect.h);
    }

    pub(crate) fn go_home(&mut self) {
        self.scroll_offset.row = 0;
        self.scroll_offset.col = 0;
        self.scroll_offset.pix_xoff = COMFY_MARGIN;
        self.scroll_offset.pix_yoff = COMFY_MARGIN;
    }
    /// Scroll so the perspective's last row is visible
    pub(crate) fn scroll_to_end(&mut self, perspective: &Perspective) {
        // Needs:
        // - row index of last byte of perspective
        // - number of rows this view can hold
        let last_row_idx = perspective.last_row_idx();
        per_msg!("{}", last_row_idx);
        self.scroll_offset.row = last_row_idx + 1;
        self.scroll_page_up();
        self.scroll_offset.floor();
        self.scroll_offset.pix_xoff = COMFY_MARGIN;
        self.scroll_offset.pix_yoff = -COMFY_MARGIN;
    }

    /// Row/col offset of relative position, including scrolling
    pub(crate) fn row_col_offset_of_pos(
        &self,
        x: i16,
        y: i16,
        perspective: &Perspective,
    ) -> Option<(usize, usize)> {
        self.viewport_rect
            .relative_offset_of_pos(x, y)
            .and_then(|(x, y)| self.row_col_of_rel_pos(x, y, perspective))
    }

    fn row_col_of_rel_pos(
        &self,
        x: i16,
        y: i16,
        perspective: &Perspective,
    ) -> Option<(usize, usize)> {
        let rel_x = x + self.scroll_offset.pix_xoff;
        let rel_y = y + self.scroll_offset.pix_yoff;
        let rel_col = rel_x / i16::from(self.col_w);
        let mut rel_row = rel_y / i16::from(self.row_h);
        if perspective.flip_row_order {
            rel_row = self.rows() - rel_row;
        }
        let row = self.scroll_offset.row;
        let col = self.scroll_offset.col;
        imm_msg!((row, col, rel_x, rel_y, rel_col, rel_row));
        if rel_x.is_positive() && rel_y.is_positive() {
            let abs_row = row + rel_row as usize;
            let abs_col = col + rel_col as usize;
            if perspective.row_col_within_bound(abs_row, abs_col) {
                Some((abs_row, abs_col))
            } else {
                None
            }
        } else {
            None
        }
    }

    pub(crate) fn center_on_offset(&mut self, offset: usize, perspective: &Perspective) {
        let (row, col) = perspective.row_col_of_byte_offset(offset);
        self.center_on_row_col(row, col);
    }

    fn center_on_row_col(&mut self, row: usize, col: usize) {
        self.scroll_offset.row = row;
        self.scroll_offset.col = col;
        self.scroll_offset.floor();
        self.scroll_x(-self.viewport_rect.w / 2);
        self.scroll_y(-self.viewport_rect.h / 2);
    }

    pub fn offsets(&self, perspective: &Perspective) -> Offsets {
        let row = self.scroll_offset.row;
        let col = self.scroll_offset.col;
        Offsets {
            row,
            col,
            byte: perspective.byte_offset_of_row_col(row, col),
        }
    }
    /// Scroll to byte offset, with control of each axis individually
    pub(crate) fn scroll_to_byte_offset(
        &mut self,
        offset: usize,
        perspective: &Perspective,
        do_col: bool,
        do_row: bool,
    ) {
        let (row, col) = perspective.row_col_of_byte_offset(offset);
        if do_row {
            self.scroll_offset.row = row;
        }
        if do_col {
            self.scroll_offset.col = col;
        }
        self.scroll_offset.floor();
    }

    pub(crate) fn bytes_per_page(&self, perspective: &Perspective) -> usize {
        self.rows() as usize * perspective.cols
    }

    /// Returns the number of rows this view can display
    pub(crate) fn rows(&self) -> i16 {
        self.viewport_rect.h / i16::from(self.row_h)
    }
}

pub struct Offsets {
    pub row: usize,
    pub col: usize,
    pub byte: usize,
}

/// It's "comfortable" to scroll a bit before the data when we're "home".
///
/// It visually indicates that we are at the beginning and there is no more data before.
const COMFY_MARGIN: i16 = -12;

/// When scrolling past 0 whole, allows unbounded negative pixel offset
fn scroll_impl(whole: &mut usize, pixel: &mut i16, pixels_per_whole: i16, scroll_by: i16) {
    *pixel += scroll_by;
    if pixel.is_negative() {
        while *pixel <= -pixels_per_whole {
            if *whole == 0 {
                break;
            }
            *whole -= 1;
            *pixel += pixels_per_whole;
        }
    } else {
        while *pixel >= pixels_per_whole {
            *whole += 1;
            *pixel -= pixels_per_whole;
        }
    }
}

#[test]
fn test_scroll_impl_positive() {
    let mut whole;
    let mut pixel;
    let px_per_whole = 32;
    // Add 1
    whole = 0;
    pixel = 0;
    scroll_impl(&mut whole, &mut pixel, px_per_whole, 1);
    assert_eq!((whole, pixel), (0, 1));
    // Add 1000
    whole = 0;
    pixel = 0;
    scroll_impl(&mut whole, &mut pixel, px_per_whole, 1000);
    assert_eq!((whole, pixel), (31, 8));
    // Add 1 until we get to 1 whole
    whole = 0;
    pixel = 0;
    for _ in 0..32 {
        scroll_impl(&mut whole, &mut pixel, px_per_whole, 1);
    }
    assert_eq!((whole, pixel), (1, 0));
}

#[test]
fn test_scroll_impl_negative() {
    let mut whole;
    let mut pixel;
    let px_per_whole = 32;
    // Add -1000 (negative test)
    whole = 0;
    pixel = 0;
    scroll_impl(&mut whole, &mut pixel, px_per_whole, -1000);
    assert_eq!((whole, pixel), (0, -1000));
    // Make 10 wholes 0
    whole = 10;
    pixel = 0;
    scroll_impl(&mut whole, &mut pixel, px_per_whole, -320);
    assert_eq!((whole, pixel), (0, 0));
    // Make 10 wholes 0, scroll remainder
    whole = 10;
    pixel = 0;
    scroll_impl(&mut whole, &mut pixel, px_per_whole, -640);
    assert_eq!((whole, pixel), (0, -320));
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ScrollOffset {
    /// What column we are at
    col: usize,
    /// Additional pixel x offset
    pix_xoff: i16,
    /// What row we are at
    row: usize,
    /// Additional pixel y offset
    pix_yoff: i16,
}

impl ScrollOffset {
    pub fn col(&self) -> usize {
        self.col
    }
    pub fn row(&self) -> usize {
        self.row
    }
    pub fn pix_xoff(&self) -> i16 {
        self.pix_xoff
    }
    pub fn pix_yoff(&self) -> i16 {
        self.pix_yoff
    }
    /// Discard pixel offsets
    pub(crate) fn floor(&mut self) {
        self.pix_xoff = 0;
        self.pix_yoff = 0;
    }
}

#[derive(Debug)]
pub struct ViewportRect {
    pub x: i16,
    pub y: i16,
    pub w: i16,
    pub h: i16,
}

/// The kind of view (hex, ascii, block, etc)
#[derive(Debug)]
pub enum ViewKind {
    Hex,
    Ascii,
    Block,
}
impl ViewportRect {
    fn relative_offset_of_pos(&self, x: i16, y: i16) -> Option<(i16, i16)> {
        self.contains_pos(x, y).then_some((x - self.x, y - self.y))
    }

    fn contains_pos(&self, x: i16, y: i16) -> bool {
        x >= self.x && y >= self.y && x <= self.x + self.w && y <= self.y + self.h
    }
}
