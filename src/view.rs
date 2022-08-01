use gamedebug_core::imm_msg;
use sfml::graphics::Font;

use crate::{
    app::{perspective::Perspective, App},
    damage_region::DamageRegion,
    edit_buffer::EditBuffer,
    hex_conv::merge_hex_halves,
    msg_if_fail, msg_warn,
};

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
    pub col_w: u16,
    /// Height of a row
    pub row_h: u16,
    /// The scrolling offset
    pub scroll_offset: ScrollOffset,
    /// The amount scrolled for a single scroll operation, in pixels
    pub scroll_speed: i16,
    /// How many bytes are required for a single block in the view
    pub bytes_per_block: u8,
    /// A view can be deactivated to not render or interact, but can later be reactivated
    pub active: bool,
    pub edit_buf: EditBuffer,
    /// The kind of text (ascii/utf16/etc)
    ///
    /// Only used by text views
    pub text_kind: TextKind,
    /// Font size
    pub font_size: u16,
    pub line_spacing: u16,
}

impl View {
    pub fn new(
        kind: ViewKind,
        x: ViewportScalar,
        y: ViewportScalar,
        w: ViewportScalar,
        h: ViewportScalar,
        font: &Font,
    ) -> Self {
        let font_size = 14;
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "It's extremely unlikely that the line spacing is not between 0..u16::MAX"
        )]
        let mut this = Self {
            viewport_rect: ViewportRect { x, y, w, h },
            kind,
            col_w: 0,
            row_h: 0,
            scroll_offset: ScrollOffset::default(),
            scroll_speed: 0,
            bytes_per_block: 1,
            active: true,
            edit_buf: EditBuffer::default(),
            text_kind: TextKind::Ascii,
            font_size,
            line_spacing: font.line_spacing(u32::from(font_size)) as u16,
        };
        this.adjust_state_to_kind();
        this
    }
    /// Used only for `mem::replace` borrow checker workarounds
    pub fn zeroed() -> Self {
        Self {
            viewport_rect: ViewportRect {
                x: 0,
                y: 0,
                w: 0,
                h: 0,
            },
            kind: ViewKind::Hex,
            col_w: 0,
            row_h: 0,
            scroll_offset: ScrollOffset::default(),
            scroll_speed: 0,
            bytes_per_block: 0,
            active: false,
            edit_buf: EditBuffer::default(),
            text_kind: TextKind::Ascii,
            font_size: 0,
            line_spacing: 0,
        }
    }
    pub fn scroll_x(&mut self, amount: i16) {
        #[expect(
            clippy::cast_possible_wrap,
            reason = "block size is never greater than i16::MAX"
        )]
        scroll_impl(
            &mut self.scroll_offset.col,
            &mut self.scroll_offset.pix_xoff,
            self.col_w as i16,
            amount,
        );
    }
    pub fn scroll_y(&mut self, amount: i16) {
        #[expect(
            clippy::cast_possible_wrap,
            reason = "block size is never greater than i16::MAX"
        )]
        scroll_impl(
            &mut self.scroll_offset.row,
            &mut self.scroll_offset.pix_yoff,
            self.row_h as i16,
            amount,
        );
    }

    pub(crate) fn sync_to(
        &mut self,
        src_row: usize,
        src_yoff: i16,
        src_col: usize,
        src_xoff: i16,
        src_row_h: u16,
        src_col_w: u16,
    ) {
        self.scroll_offset.row = src_row;
        self.scroll_offset.col = src_col;
        let y_ratio = f64::from(src_row_h) / f64::from(self.row_h);
        let x_ratio = f64::from(src_col_w) / f64::from(self.col_w);
        #[expect(
            clippy::cast_possible_truncation,
            reason = "Input values are all low (look at input types)"
        )]
        {
            self.scroll_offset.pix_yoff = (f64::from(src_yoff) / y_ratio) as i16;
            self.scroll_offset.pix_xoff = (f64::from(src_xoff) / x_ratio) as i16;
        }
    }

    pub(crate) fn scroll_page_down(&mut self) {
        self.scroll_y(self.viewport_rect.h);
    }

    pub(crate) fn scroll_page_up(&mut self) {
        self.scroll_y(-self.viewport_rect.h);
    }

    pub(crate) fn scroll_page_left(&mut self) {
        self.scroll_x(-self.viewport_rect.w);
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
        let last_col_idx = perspective.last_col_idx();
        self.scroll_offset.row = last_row_idx + 1;
        self.scroll_offset.col = last_col_idx + 1;
        self.scroll_page_up();
        self.scroll_page_left();
        self.scroll_offset.floor();
        self.scroll_offset.pix_xoff = -COMFY_MARGIN;
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
    #[expect(
        clippy::cast_possible_wrap,
        reason = "block size is never greater than i16::MAX"
    )]
    fn row_col_of_rel_pos(
        &self,
        x: i16,
        y: i16,
        perspective: &Perspective,
    ) -> Option<(usize, usize)> {
        let rel_x = x + self.scroll_offset.pix_xoff;
        let rel_y = y + self.scroll_offset.pix_yoff;
        let rel_col = rel_x / self.col_w as i16;
        let mut rel_row = rel_y / self.row_h as i16;
        if perspective.flip_row_order {
            rel_row = self.rows() - rel_row;
        }
        let row = self.scroll_offset.row;
        let col = self.scroll_offset.col;
        imm_msg!((row, col, rel_x, rel_y, rel_col, rel_row));
        #[expect(
            clippy::cast_sign_loss,
            reason = "rel_x and rel_y being positive also ensure rel_row and rel_col are"
        )]
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

    pub const fn offsets(&self, perspective: &Perspective) -> Offsets {
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
    #[expect(
        clippy::cast_sign_loss,
        reason = "View::rows() being negative is a bug, can expect positive."
    )]
    pub(crate) const fn bytes_per_page(&self, perspective: &Perspective) -> usize {
        self.rows() as usize * perspective.cols
    }

    /// Returns the number of rows this view can display
    #[expect(
        clippy::cast_possible_wrap,
        reason = "block size is never greater than i16::MAX"
    )]
    pub(crate) const fn rows(&self) -> i16 {
        self.viewport_rect.h / self.row_h as i16
    }

    pub fn adjust_block_size(&mut self) {
        (self.col_w, self.row_h) = match self.kind {
            ViewKind::Hex => (self.font_size * 2 - 2, self.font_size),
            ViewKind::Dec => (self.font_size * 3 - 6, self.font_size),
            ViewKind::Text => (self.font_size, self.line_spacing.max(1)),
            ViewKind::Block => (4, 4),
        }
    }
    /// Adjust state after kind was changed
    pub fn adjust_state_to_kind(&mut self) {
        self.adjust_block_size();
        let glyph_count = self.glyph_count();
        self.edit_buf.resize(glyph_count);
    }
    /// The number of glyphs per block this view has
    const fn glyph_count(&self) -> u16 {
        match self.kind {
            ViewKind::Hex => 2,
            ViewKind::Dec => 3,
            ViewKind::Text | ViewKind::Block => 1,
        }
    }
    pub fn handle_text_entered(&mut self, unicode: char, app: &mut App) {
        if self.char_valid(unicode) {
            if !self.edit_buf.dirty {
                match self.kind {
                    ViewKind::Hex => {
                        let s = format!("{:02X}", app.data[app.edit_state.cursor]);
                        self.edit_buf.update_from_string(&s);
                    }
                    ViewKind::Dec => {
                        let s = format!("{:03}", app.data[app.edit_state.cursor]);
                        self.edit_buf.update_from_string(&s);
                    }
                    // Ascii doesn't need any copy buffer updates because it only ever deals with
                    // one glyph at a time
                    // Block doesn't do any text input
                    ViewKind::Text | ViewKind::Block => {}
                }
            }
            if self.edit_buf.enter_byte(unicode.to_ascii_uppercase() as u8)
                || app.preferences.quick_edit
            {
                self.finish_editing(app);
            }
        }
    }

    const fn char_valid(&self, unicode: char) -> bool {
        match self.kind {
            ViewKind::Hex => matches!(unicode, '0'..='9' | 'a'..='f'),
            ViewKind::Dec => matches!(unicode, '0'..='9'),
            ViewKind::Text => unicode.is_ascii(),
            ViewKind::Block => false,
        }
    }

    pub fn finish_editing(&mut self, app: &mut App) {
        match self.kind {
            ViewKind::Hex => {
                app.data[app.edit_state.cursor] =
                    merge_hex_halves(self.edit_buf.buf[0], self.edit_buf.buf[1]);
                app.widen_dirty_region(&DamageRegion::Single(app.edit_state.cursor));
            }
            ViewKind::Dec => {
                let s =
                    std::str::from_utf8(&self.edit_buf.buf).expect("Invalid utf-8 in edit buffer");
                match s.parse() {
                    Ok(num) => {
                        app.data[app.edit_state.cursor] = num;
                        app.widen_dirty_region(&DamageRegion::Single(app.edit_state.cursor));
                    }
                    Err(e) => msg_warn(&format!("Invalid value: {}", e)),
                }
            }
            ViewKind::Text => {
                app.data[app.edit_state.cursor] = self.edit_buf.buf[0];
                app.widen_dirty_region(&DamageRegion::Single(app.edit_state.cursor));
            }
            ViewKind::Block => {}
        }
        if app.edit_state.cursor + 1 < app.data.len() && !app.preferences.sticky_edit {
            app.edit_state.step_cursor_forward();
        }
        self.edit_buf.reset();

        if app.preferences.auto_save {
            msg_if_fail(app.save(), "Failed to save file");
        }
    }

    pub fn cancel_editing(&mut self) {
        self.edit_buf.reset();
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
    pub const fn col(&self) -> usize {
        self.col
    }
    pub const fn row(&self) -> usize {
        self.row
    }
    pub const fn pix_xoff(&self) -> i16 {
        self.pix_xoff
    }
    pub const fn pix_yoff(&self) -> i16 {
        self.pix_yoff
    }
    /// Discard pixel offsets
    pub(crate) fn floor(&mut self) {
        self.pix_xoff = 0;
        self.pix_yoff = 0;
    }
}

/// Type for representing viewport magnitudes.
///
/// We assume that hexerator will never run on resolutions higher than 32767x32767,
/// or get mouse positions higher than that.
pub type ViewportScalar = i16;

#[derive(Debug)]
pub struct ViewportRect {
    pub x: ViewportScalar,
    pub y: ViewportScalar,
    pub w: ViewportScalar,
    pub h: ViewportScalar,
}

#[derive(Debug, Copy, Clone)]
pub struct ViewportVec {
    pub x: ViewportScalar,
    pub y: ViewportScalar,
}

impl TryFrom<sfml::system::Vector2<i32>> for ViewportVec {
    type Error = <ViewportScalar as std::convert::TryFrom<i32>>::Error;

    fn try_from(sf_vec: sfml::system::Vector2<i32>) -> Result<Self, Self::Error> {
        Ok(Self {
            x: sf_vec.x.try_into()?,
            y: sf_vec.y.try_into()?,
        })
    }
}

impl TryFrom<(i32, i32)> for ViewportVec {
    type Error = <ViewportScalar as std::convert::TryFrom<i32>>::Error;

    fn try_from(src: (i32, i32)) -> Result<Self, Self::Error> {
        Ok(Self {
            x: src.0.try_into()?,
            y: src.1.try_into()?,
        })
    }
}

/// The kind of view (hex, ascii, block, etc)
#[derive(Debug, PartialEq, Eq)]
pub enum ViewKind {
    Hex,
    Dec,
    Text,
    Block,
}

#[derive(Debug, PartialEq, Eq)]
pub enum TextKind {
    Ascii,
    Utf16Le,
    Utf16Be,
}

impl TextKind {
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Ascii => "ascii",
            Self::Utf16Le => "utf-16 le",
            Self::Utf16Be => "utf-16 be",
        }
    }

    pub(crate) const fn bytes_needed(&self) -> u8 {
        match self {
            Self::Ascii => 1,
            Self::Utf16Le | Self::Utf16Be => 2,
        }
    }
}

impl ViewportRect {
    fn relative_offset_of_pos(
        &self,
        x: ViewportScalar,
        y: ViewportScalar,
    ) -> Option<(ViewportScalar, ViewportScalar)> {
        self.contains_pos(x, y).then_some((x - self.x, y - self.y))
    }

    const fn contains_pos(&self, x: ViewportScalar, y: ViewportScalar) -> bool {
        x >= self.x && y >= self.y && x <= self.x + self.w && y <= self.y + self.h
    }
}
