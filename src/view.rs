use {
    crate::{
        app::{edit_state::EditState, presentation::Presentation},
        damage_region::DamageRegion,
        data::Data,
        edit_buffer::EditBuffer,
        gui::message_dialog::{Icon, MessageDialog},
        hex_conv::merge_hex_halves,
        meta::{MetaLow, PerspectiveKey, PerspectiveMap, RegionMap, region::Region},
        session_prefs::SessionPrefs,
    },
    gamedebug_core::per,
    serde::{Deserialize, Serialize},
    slotmap::Key as _,
};

mod draw;

/// A rectangular view in the viewport looking through a perspective at the data with a flavor
/// of rendering/interaction (hex/ascii/block/etc.)
///
/// There can be different views through the same perspective.
/// By default they sync their offsets, but each view can show different amounts of data
/// depending on block size of its items, and its relative size in the viewport.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct View {
    /// The rectangle to occupy in the viewport
    #[serde(skip)]
    pub viewport_rect: ViewportRect,
    /// The kind of view (hex, ascii, block, etc)
    pub kind: ViewKind,
    /// Width of a column
    pub col_w: u16,
    /// Height of a row
    pub row_h: u16,
    /// The scrolling offset
    #[serde(skip)]
    pub scroll_offset: ScrollOffset,
    /// The amount scrolled for a single scroll operation, in pixels
    pub scroll_speed: i16,
    /// How many bytes are required for a single block in the view
    pub bytes_per_block: u8,
    /// The perspective this view is associated with
    pub perspective: PerspectiveKey,
    /// Color schemes, etc.
    pub presentation: Presentation,
}

impl PartialEq for View {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind
            && self.col_w == other.col_w
            && self.row_h == other.row_h
            && self.scroll_speed == other.scroll_speed
            && self.bytes_per_block == other.bytes_per_block
            && self.presentation == other.presentation
    }
}

impl Eq for View {}

impl View {
    pub fn new(kind: ViewKind, perspective: PerspectiveKey) -> Self {
        let mut this = Self {
            viewport_rect: ViewportRect::default(),
            kind,
            // TODO: Hack. We're setting this to 4, 4 to avoid zeroed default block view.
            // Solve in a better way.
            col_w: 4,
            row_h: 4,
            scroll_offset: ScrollOffset::default(),
            scroll_speed: 0,
            bytes_per_block: 1,
            perspective,
            presentation: Presentation::default(),
        };
        this.adjust_state_to_kind();
        this
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
        self.scroll_offset.floor();
    }

    pub(crate) fn go_home_col(&mut self) {
        self.scroll_offset.col = 0;
        self.scroll_offset.pix_xoff = 0;
    }

    /// Scroll so the perspective's last row is visible
    pub(crate) fn scroll_to_end(&mut self, meta_low: &MetaLow) {
        // Needs:
        // - row index of last byte of perspective
        // - number of rows this view can hold
        let perspective = &meta_low.perspectives[self.perspective];
        let last_row_idx = perspective.last_row_idx(&meta_low.regions);
        let last_col_idx = perspective.last_col_idx(&meta_low.regions);
        self.scroll_offset.row = last_row_idx + 1;
        self.scroll_offset.col = last_col_idx + 1;
        self.scroll_page_up();
        self.scroll_page_left();
        self.scroll_offset.floor();
    }
    /// Scrolls the view right until it "bumps" into the right edge of content
    pub(crate) fn scroll_right_until_bump(&mut self, meta_low: &MetaLow) {
        let per = &meta_low.perspectives[self.perspective];
        #[expect(clippy::cast_sign_loss, reason = "self.cols() is essentially `u15`")]
        let view_cols = self.cols() as usize;
        let offset = per.cols.saturating_sub(view_cols);
        self.scroll_offset.col = offset;
        self.scroll_offset.floor();
    }

    /// Row/col offset of relative position, including scrolling
    pub(crate) fn row_col_offset_of_pos(
        &self,
        x: i16,
        y: i16,
        perspectives: &PerspectiveMap,
        regions: &RegionMap,
    ) -> Option<[usize; 2]> {
        self.viewport_rect
            .relative_offset_of_pos(x, y)
            .and_then(|(x, y)| self.row_col_of_rel_pos(x, y, perspectives, regions))
    }
    #[expect(
        clippy::cast_possible_wrap,
        reason = "block size is never greater than i16::MAX"
    )]
    fn row_col_of_rel_pos(
        &self,
        x: i16,
        y: i16,
        perspectives: &PerspectiveMap,
        regions: &RegionMap,
    ) -> Option<[usize; 2]> {
        let rel_x = x + self.scroll_offset.pix_xoff;
        let rel_y = y + self.scroll_offset.pix_yoff;
        let rel_col = rel_x / self.col_w as i16;
        let mut rel_row = rel_y / self.row_h as i16;
        let perspective = match perspectives.get(self.perspective) {
            Some(per) => per,
            None => {
                per!("row_col_of_rel_pos: Invalid perspective key");
                return None;
            }
        };
        if perspective.flip_row_order {
            rel_row = self.rows() - rel_row;
        }
        let row = self.scroll_offset.row;
        let col = self.scroll_offset.col;
        #[expect(
            clippy::cast_sign_loss,
            reason = "rel_x and rel_y being positive also ensure rel_row and rel_col are"
        )]
        if rel_x.is_positive() && rel_y.is_positive() {
            let abs_row = row + rel_row as usize;
            let abs_col = col + rel_col as usize;
            if perspective.row_col_within_bound(abs_row, abs_col, regions) {
                Some([abs_row, abs_col])
            } else {
                None
            }
        } else {
            None
        }
    }

    pub(crate) fn center_on_offset(
        &mut self,
        offset: usize,
        perspectives: &PerspectiveMap,
        regions: &RegionMap,
    ) {
        let [row, col] = perspectives[self.perspective].row_col_of_byte_offset(offset, regions);
        self.center_on_row_col(row, col);
    }

    fn center_on_row_col(&mut self, row: usize, col: usize) {
        self.scroll_offset.row = row;
        self.scroll_offset.col = col;
        self.scroll_offset.floor();
        self.scroll_x(-self.viewport_rect.w / 2);
        self.scroll_y(-self.viewport_rect.h / 2);
    }

    pub fn offsets(&self, perspectives: &PerspectiveMap, regions: &RegionMap) -> Offsets {
        let row = self.scroll_offset.row;
        let col = self.scroll_offset.col;
        Offsets {
            row,
            col,
            byte: perspectives[self.perspective].byte_offset_of_row_col(row, col, regions),
        }
    }
    /// Scroll to byte offset, with control of each axis individually
    pub(crate) fn scroll_to_byte_offset(
        &mut self,
        offset: usize,
        perspectives: &PerspectiveMap,
        regions: &RegionMap,
        do_col: bool,
        do_row: bool,
    ) {
        let [row, col] = perspectives[self.perspective].row_col_of_byte_offset(offset, regions);
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
    pub(crate) fn bytes_per_page(&self, perspectives: &PerspectiveMap) -> usize {
        (self.rows() as usize) * perspectives[self.perspective].cols
    }

    /// Returns the number of rows this view can display
    #[expect(
        clippy::cast_possible_wrap,
        reason = "block size is never greater than i16::MAX"
    )]
    pub(crate) fn rows(&self) -> i16 {
        // If the viewport rect is smaller than 0, we just return 0 for the rows
        if self.viewport_rect.h <= 0 {
            return 0;
        }
        self.viewport_rect.h / (self.row_h as i16)
    }
    /// Returns the number of columns this view can display visibly at once.
    ///
    /// This might not be the total number of columns in the perspective this view is attached to.
    #[expect(
        clippy::cast_possible_wrap,
        reason = "block size is never greater than i16::MAX"
    )]
    pub(crate) fn cols(&self) -> i16 {
        match self.viewport_rect.w.checked_div(self.col_w as i16) {
            Some(result) => result,
            None => {
                per!("Divide by zero in View::cols. Bug.");
                0
            }
        }
    }

    /// Returns the number of columns of the perspective this view is attached to.
    pub(crate) fn p_cols(&self, perspectives: &PerspectiveMap) -> usize {
        match perspectives.get(self.perspective) {
            Some(per) => per.cols,
            None => 0,
        }
    }

    pub fn adjust_block_size(&mut self) {
        (self.col_w, self.row_h) = match &self.kind {
            ViewKind::Hex(hex) => (hex.font_size * 2 - 2, hex.font_size),
            ViewKind::Dec(dec) => (dec.font_size * 3 - 6, dec.font_size),
            ViewKind::Text(data) => (data.font_size, data.line_spacing.max(1)),
            ViewKind::Block => (self.col_w, self.row_h),
        }
    }
    /// Adjust state after kind was changed
    pub fn adjust_state_to_kind(&mut self) {
        self.adjust_block_size();
        let glyph_count = self.glyph_count();
        match &mut self.kind {
            ViewKind::Hex(HexData { edit_buf, .. })
            | ViewKind::Dec(HexData { edit_buf, .. })
            | ViewKind::Text(TextData { edit_buf, .. }) => edit_buf.resize(glyph_count),
            _ => {}
        }
    }
    /// The number of glyphs per block this view has
    fn glyph_count(&self) -> u16 {
        match self.kind {
            ViewKind::Hex(_) => 2,
            ViewKind::Dec(_) => 3,
            ViewKind::Text { .. } => 1,
            ViewKind::Block => 1,
        }
    }
    pub fn handle_text_entered(
        &mut self,
        unicode: char,
        edit_state: &mut EditState,
        preferences: &SessionPrefs,
        data: &mut Data,
        msg: &mut MessageDialog,
    ) {
        if self.char_valid(unicode) {
            match &mut self.kind {
                ViewKind::Hex(hex) => {
                    if !hex.edit_buf.dirty {
                        let Some(byte) = data.get(edit_state.cursor) else {
                            return;
                        };
                        let s = format!("{byte:02X}");
                        hex.edit_buf.update_from_string(&s);
                    }
                    if hex.edit_buf.enter_byte(unicode.to_ascii_uppercase() as u8)
                        || preferences.quick_edit
                    {
                        self.finish_editing(edit_state, data, preferences, msg);
                    }
                }
                ViewKind::Dec(dec) => {
                    if !dec.edit_buf.dirty {
                        let Some(byte) = data.get(edit_state.cursor) else {
                            return;
                        };
                        let s = format!("{byte:03}");
                        dec.edit_buf.update_from_string(&s);
                    }
                    if dec.edit_buf.enter_byte(unicode.to_ascii_uppercase() as u8)
                        || preferences.quick_edit
                    {
                        self.finish_editing(edit_state, data, preferences, msg);
                    }
                }
                ViewKind::Text(text) => {
                    if text.edit_buf.enter_byte((unicode as u8).wrapping_add_signed(-text.offset))
                        || preferences.quick_edit
                    {
                        self.finish_editing(edit_state, data, preferences, msg);
                    }
                }
                // Block doesn't do any text input
                ViewKind::Block => {}
            }
        }
    }

    /// Returns the size needed by this view to display fully
    pub fn max_needed_size(
        &self,
        perspectives: &PerspectiveMap,
        regions: &RegionMap,
    ) -> ViewportVec {
        if self.perspective.is_null() {
            return ViewportVec { x: 0, y: 0 };
        }
        let p = &perspectives[self.perspective];
        let n_rows = p.n_rows(regions);
        ViewportVec {
            x: i16::saturating_from(p.cols).saturating_mul(i16::saturating_from(self.col_w)),
            y: i16::saturating_from(n_rows).saturating_mul(i16::saturating_from(self.row_h)),
        }
    }

    fn char_valid(&self, unicode: char) -> bool {
        match self.kind {
            ViewKind::Hex(_) => matches!(unicode, '0'..='9' | 'a'..='f'),
            ViewKind::Dec(_) => unicode.is_ascii_digit(),
            ViewKind::Text { .. } => {
                unicode.is_ascii() && !unicode.is_control() && !matches!(unicode, '\t')
            }
            ViewKind::Block => false,
        }
    }

    pub fn finish_editing(
        &mut self,
        edit_state: &mut EditState,
        data: &mut Data,
        preferences: &SessionPrefs,
        msg: &mut MessageDialog,
    ) {
        match &mut self.kind {
            ViewKind::Hex(hex) => {
                match merge_hex_halves(hex.edit_buf.buf[0], hex.edit_buf.buf[1]) {
                    Some(merged) => {
                        if let Some(byte) = data.get_mut(edit_state.cursor) {
                            *byte = merged;
                        }
                    }
                    None => per!("finish_editing: Failed to merge hex halves"),
                }
                data.widen_dirty_region(DamageRegion::Single(edit_state.cursor));
            }
            ViewKind::Dec(dec) => {
                let s =
                    std::str::from_utf8(&dec.edit_buf.buf).expect("Invalid utf-8 in edit buffer");
                match s.parse() {
                    Ok(num) => {
                        data[edit_state.cursor] = num;
                        data.widen_dirty_region(DamageRegion::Single(edit_state.cursor));
                    }
                    Err(e) => msg.open(Icon::Error, "Invalid value", e.to_string()),
                }
            }
            ViewKind::Text(text) => {
                let Some(byte) = data.get_mut(edit_state.cursor) else {
                    return;
                };
                *byte = text.edit_buf.buf[0];
                data.widen_dirty_region(DamageRegion::Single(edit_state.cursor));
            }
            ViewKind::Block => {}
        }
        if edit_state.cursor + 1 < data.len() && !preferences.sticky_edit {
            edit_state.step_cursor_forward();
        }
        self.reset_edit_buf();
    }

    pub fn cancel_editing(&mut self) {
        self.reset_edit_buf();
    }

    pub fn reset_edit_buf(&mut self) {
        if let Some(edit_buf) = self.edit_buffer_mut() {
            edit_buf.reset();
        }
    }

    pub(crate) fn undirty_edit_buffer(&mut self) {
        if let Some(edit_buf) = self.edit_buffer_mut() {
            edit_buf.dirty = false;
        }
    }

    pub(crate) fn edit_buffer_mut(&mut self) -> Option<&mut EditBuffer> {
        match &mut self.kind {
            ViewKind::Hex(data) | ViewKind::Dec(data) => Some(&mut data.edit_buf),
            ViewKind::Text(data) => Some(&mut data.edit_buf),
            ViewKind::Block => None,
        }
    }

    pub(crate) fn contains_region(&self, reg: &Region, meta: &crate::meta::Meta) -> bool {
        meta.low.regions[meta.low.perspectives[self.perspective].region]
            .region
            .contains_region(reg)
    }
}

trait SatFrom<V> {
    fn saturating_from(src: V) -> Self;
}

impl SatFrom<usize> for i16 {
    fn saturating_from(src: usize) -> Self {
        Self::try_from(src).unwrap_or(Self::MAX)
    }
}

impl SatFrom<u16> for i16 {
    fn saturating_from(src: u16) -> Self {
        Self::try_from(src).unwrap_or(Self::MAX)
    }
}

pub struct Offsets {
    pub row: usize,
    pub col: usize,
    pub byte: usize,
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

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
pub struct ScrollOffset {
    /// What column we are at
    pub col: usize,
    /// Additional pixel x offset
    pub pix_xoff: i16,
    /// What row we are at
    pub row: usize,
    /// Additional pixel y offset
    pub pix_yoff: i16,
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

/// Type for representing viewport magnitudes.
///
/// We assume that hexerator will never run on resolutions higher than 32767x32767,
/// or get mouse positions higher than that.
pub type ViewportScalar = i16;

#[derive(Debug, Default, Clone, Copy)]
pub struct ViewportRect {
    pub x: ViewportScalar,
    pub y: ViewportScalar,
    pub w: ViewportScalar,
    pub h: ViewportScalar,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct ViewportVec {
    pub x: ViewportScalar,
    pub y: ViewportScalar,
}

impl TryFrom<(i32, i32)> for ViewportVec {
    type Error = <ViewportScalar as TryFrom<i32>>::Error;

    fn try_from(src: (i32, i32)) -> Result<Self, Self::Error> {
        Ok(Self {
            x: src.0.try_into()?,
            y: src.1.try_into()?,
        })
    }
}

/// The kind of view (hex, ascii, block, etc)
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum ViewKind {
    Hex(HexData),
    Dec(HexData),
    Text(TextData),
    Block,
}

impl ViewKind {
    pub(crate) fn is_text(&self) -> bool {
        matches!(self, Self::Text(_))
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TextData {
    /// The kind of text (ascii/utf16/etc)
    pub text_kind: TextKind,
    pub line_spacing: u16,
    #[serde(skip)]
    pub edit_buf: EditBuffer,
    pub font_size: u16,
    /// Offset from regular ascii offsets. Useful to see custom (single byte) text encodings
    #[serde(default)]
    pub offset: i8,
}

impl PartialEq for TextData {
    fn eq(&self, other: &Self) -> bool {
        self.text_kind == other.text_kind
            && self.line_spacing == other.line_spacing
            && self.font_size == other.font_size
    }
}

impl Eq for TextData {}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HexData {
    #[serde(skip)]
    pub edit_buf: EditBuffer,
    pub font_size: u16,
}

impl PartialEq for HexData {
    fn eq(&self, other: &Self) -> bool {
        self.font_size == other.font_size
    }
}

impl Eq for HexData {}

impl HexData {
    pub fn with_font_size(font_size: u16) -> Self {
        Self {
            edit_buf: Default::default(),
            font_size,
        }
    }
}

impl TextData {
    pub fn with_font_info(line_spacing: u16, font_size: u16) -> Self {
        Self {
            text_kind: TextKind::Ascii,
            line_spacing,
            edit_buf: EditBuffer::default(),
            font_size,
            offset: 0,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub enum TextKind {
    Ascii,
    Utf16Le,
    Utf16Be,
}

impl TextKind {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Ascii => "ascii",
            Self::Utf16Le => "utf-16 le",
            Self::Utf16Be => "utf-16 be",
        }
    }

    pub(crate) fn bytes_needed(&self) -> u8 {
        match self {
            Self::Ascii => 1,
            Self::Utf16Le => 2,
            Self::Utf16Be => 2,
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

    pub fn contains_pos(&self, x: ViewportScalar, y: ViewportScalar) -> bool {
        x >= self.x && y >= self.y && x <= self.x + self.w && y <= self.y + self.h
    }
}

/// Try to convert mouse position to ViewportVec.
///
/// Log error and return zeroed vec on conversion error.
pub fn try_conv_mp_zero<T: TryInto<ViewportVec>>(src: T) -> ViewportVec
where
    T::Error: std::fmt::Display,
{
    match src.try_into() {
        Ok(mp) => mp,
        Err(e) => {
            per!(
                "Mouse position conversion error: {}\nHexerator doesn't support extremely high (>32700) mouse positions.",
                e
            );
            ViewportVec { x: 0, y: 0 }
        }
    }
}
