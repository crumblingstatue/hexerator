use std::cmp::{max, min};

use serde::{Deserialize, Serialize};

use crate::{
    app::{PerspectiveMap, RegionMap, ViewKey, ViewMap},
    view::ViewportRect,
};

/// A view layout grid for laying out views.
#[derive(Clone, Serialize, Deserialize)]
pub struct Layout {
    pub name: String,
    pub view_grid: Vec<Vec<ViewKey>>,
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
    let mut x_cursor = hex_iface_rect.x;
    let mut y_cursor = hex_iface_rect.y;
    for row in &layout.view_grid {
        let max_allowed_h = hex_iface_rect.h / layout.view_grid.len() as i16;
        let mut max_h = 0;
        for &view_key in row {
            let max_allowed_w = hex_iface_rect.w / row.len() as i16;
            let view = &mut view_map[view_key].view;
            let max_needed_size = view.max_needed_size(perspectives, regions);
            let w = min(max_needed_size.x, max_allowed_w);
            let h = min(max_needed_size.y, max_allowed_h);
            view.viewport_rect.x = x_cursor;
            view.viewport_rect.y = y_cursor;
            view.viewport_rect.w = w;
            view.viewport_rect.h = h;
            max_h = max(max_h, h);
            x_cursor += w;
        }
        x_cursor = hex_iface_rect.x;
        y_cursor += max_h;
    }
}
