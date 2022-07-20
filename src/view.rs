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

#[derive(Debug)]
pub struct ScrollOffset {
    /// What column we are at
    pub col_x: usize,
    /// Additional pixel x offset
    pub pix_x: i16,
    /// What row we are at
    pub row_y: usize,
    /// Additional pixel y offset
    pub pix_y: i16,
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
