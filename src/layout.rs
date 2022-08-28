use std::cmp::{max, min};

use serde::{Deserialize, Serialize};

use crate::{
    app::{PerspectiveMap, RegionMap, ViewKey, ViewMap},
    view::{ViewportRect, ViewportScalar},
};

/// A view layout grid for laying out views.
#[derive(Clone, Serialize, Deserialize)]
pub struct Layout {
    pub name: String,
    pub view_grid: Vec<Vec<ViewKey>>,
    /// Margin around views
    #[serde(default = "default_margin")]
    pub margin: ViewportScalar,
}

pub const fn default_margin() -> ViewportScalar {
    6
}

impl Layout {
    /// Iterate through all view keys
    pub fn iter(&self) -> impl Iterator<Item = ViewKey> + '_ {
        self.view_grid.iter().flatten().cloned()
    }
}

pub fn do_auto_layout(
    layout: &Layout,
    view_map: &mut ViewMap,
    hex_iface_rect: &ViewportRect,
    perspectives: &PerspectiveMap,
    regions: &RegionMap,
) {
    let mut x_cursor = hex_iface_rect.x + layout.margin;
    let mut y_cursor = hex_iface_rect.y + layout.margin;
    let layout_n_rows = i16::try_from(layout.view_grid.len()).expect("Too many rows in layout");
    for row in &layout.view_grid {
        let max_allowed_h =
            (hex_iface_rect.h - (layout.margin * (layout_n_rows + 1))) / layout_n_rows;
        let mut max_h = 0;
        let row_n_cols = i16::try_from(row.len()).expect("Too many columns in layout");
        for &view_key in row {
            let max_allowed_w =
                (hex_iface_rect.w - (layout.margin * (row_n_cols + 1))) / row_n_cols;
            let view = &mut view_map[view_key].view;
            let max_needed_size = view.max_needed_size(perspectives, regions);
            let w = min(max_needed_size.x, max_allowed_w);
            let h = min(max_needed_size.y, max_allowed_h);
            view.viewport_rect.x = x_cursor;
            view.viewport_rect.y = y_cursor;
            view.viewport_rect.w = w;
            view.viewport_rect.h = h;
            max_h = max(max_h, h);
            x_cursor += w + layout.margin;
        }
        x_cursor = hex_iface_rect.x + layout.margin;
        y_cursor += max_h + layout.margin;
    }
}
