use {
    crate::{
        app::interact_mode::InteractMode,
        color::RgbaColor,
        meta::{LayoutKey, ViewKey, region::Region},
        timer::Timer,
        view::ViewportRect,
    },
    slotmap::Key as _,
    std::{collections::HashMap, time::Duration},
};

/// State related to the hex view ui, different from the egui gui overlay
#[derive(Default)]
pub struct HexUi {
    /// "a" point of selection. Could be smaller or larger than "b".
    /// The length of selection is absolute difference between a and b
    pub select_a: Option<usize>,
    /// "b" point of selection. Could be smaller or larger than "a".
    /// The length of selection is absolute difference between a and b
    pub select_b: Option<usize>,
    /// Extra selections on top of the a-b selection
    pub extra_selections: Vec<Region>,
    pub interact_mode: InteractMode = InteractMode::View,
    pub current_layout: LayoutKey,
    /// The currently focused view (appears with a yellow border around it)
    #[doc(alias = "current_view")]
    pub focused_view: Option<ViewKey>,
    /// The rectangle area that's available for the hex interface
    pub hex_iface_rect: ViewportRect,
    pub flash_cursor_timer: Timer,
    /// Whether to scissor views when drawing them. Useful to disable when debugging rendering.
    pub scissor_views: bool = true,
    /// When alt is being held, it shows things like names of views as overlays
    pub show_alt_overlay: bool,
    pub rulers: HashMap<ViewKey, Ruler>,
    /// If `Some`, contains the last byte offset the cursor was clicked at, while lmb is being held down
    pub lmb_drag_offset: Option<usize>,
}

#[derive(Default)]
pub struct Ruler {
    pub color: RgbaColor = RgbaColor { r: 255, g: 255, b: 0,a: 255},
    /// Horizontal offset in pixels
    pub hoffset: i16,
    /// Frequency of ruler lines
    pub freq: u8 = 1,
    /// If set, it will try to layout ruler based on the struct fields
    pub struct_idx: Option<usize>,
}

impl HexUi {
    pub fn selection(&self) -> Option<Region> {
        if let Some(a) = self.select_a
            && let Some(b) = self.select_b
        {
            Some(Region {
                begin: a.min(b),
                end: a.max(b),
            })
        } else {
            None
        }
    }
    pub fn selected_regions(&self) -> impl Iterator<Item = Region> {
        self.selection().into_iter().chain(self.extra_selections.iter().cloned())
    }
    pub fn clear_selections(&mut self) {
        self.select_a = None;
        self.select_b = None;
        self.extra_selections.clear();
    }
    /// Clear existing meta references
    pub fn clear_meta_refs(&mut self) {
        self.current_layout = LayoutKey::null();
        self.focused_view = None;
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
        self.flash_cursor_timer.overtime().map(|dur| dur.as_millis() as u32)
    }
}
