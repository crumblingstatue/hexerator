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
    }
}

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
