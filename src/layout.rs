use {
    crate::{
        meta::{region::Region, PerspectiveMap, RegionMap, ViewKey, ViewMap},
        view::{ViewportRect, ViewportScalar},
    },
    serde::{Deserialize, Serialize},
    std::cmp::{max, min},
};

/// A view layout grid for laying out views.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
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

    pub(crate) fn idx_of_key(&self, key: ViewKey) -> Option<(usize, usize)> {
        self.view_grid.iter().enumerate().find_map(|(row_idx, row)| {
            let col_pos = row.iter().position(|k| *k == key)?;
            Some((row_idx, col_pos))
        })
    }

    pub(crate) fn view_containing_region(
        &self,
        reg: &Region,
        meta: &crate::meta::Meta,
    ) -> Option<ViewKey> {
        self.iter()
            .find(|view_key| meta.views[*view_key].view.contains_region(reg, meta))
    }

    pub(crate) fn contains_view(&self, key: ViewKey) -> bool {
        self.iter().any(|k| k == key)
    }

    pub(crate) fn remove_view(&mut self, rem_key: ViewKey) {
        self.view_grid.retain_mut(|row| {
            row.retain(|view_key| *view_key != rem_key);
            !row.is_empty()
        });
    }

    pub(crate) fn remove_dangling(&mut self, map: &ViewMap) {
        self.view_grid.retain_mut(|row| {
            row.retain(|view_key| {
                let mut retain = true;
                if !map.contains_key(*view_key) {
                    eprintln!(
                        "Removed dangling view {:?} from layout {}",
                        view_key, self.name
                    );
                    retain = false;
                }
                retain
            });
            !row.is_empty()
        });
    }

    pub(crate) fn change_view_type(&mut self, current: ViewKey, new: ViewKey) {
        if let Some(current_key) = self.view_grid.iter_mut().flatten().find(|k| **k == current) {
            *current_key = new;
        }
    }
}

pub fn do_auto_layout(
    layout: &Layout,
    view_map: &mut ViewMap,
    hex_iface_rect: &ViewportRect,
    perspectives: &PerspectiveMap,
    regions: &RegionMap,
) {
    let layout_n_rows = i16::try_from(layout.view_grid.len()).expect("Too many rows in layout");
    let mut total_h = 0;
    // Determine sizes
    for row in &layout.view_grid {
        let max_allowed_h =
            (hex_iface_rect.h - (layout.margin * (layout_n_rows + 1))) / layout_n_rows;
        let row_n_cols = i16::try_from(row.len()).expect("Too many columns in layout");
        let mut total_row_w = 0;
        let mut max_h = 0;
        for &view_key in row {
            let max_allowed_w =
                (hex_iface_rect.w - (layout.margin * (row_n_cols + 1))) / row_n_cols;
            let view = &mut view_map[view_key].view;
            let max_needed_size = view.max_needed_size(perspectives, regions);
            let w = min(max_needed_size.x, max_allowed_w);
            let h = min(max_needed_size.y, max_allowed_h);
            view.viewport_rect.w = w;
            total_row_w += w;
            view.viewport_rect.h = h;
            max_h = max(max_h, view.viewport_rect.h);
        }
        total_h += max_h;
        // Distribute remaining width to views in order
        let w_to_fill_viewport = hex_iface_rect.w - (layout.margin * (row_n_cols + 1));
        let mut w_remaining = w_to_fill_viewport - total_row_w;
        for &view_key in row {
            if w_remaining <= 0 {
                break;
            }
            let view = &mut view_map[view_key].view;
            let max_needed_w = view.max_needed_size(perspectives, regions).x;
            let missing_for_max_needed = max_needed_w - view.viewport_rect.w;
            let can_add = min(missing_for_max_needed, w_remaining);
            view.viewport_rect.w += can_add;
            w_remaining -= can_add;
        }
    }
    // Distribute remaining height to rows in order
    let h_to_fill_viewport = hex_iface_rect.h - (layout.margin * (layout_n_rows + 1));
    let mut h_remaining = h_to_fill_viewport - total_h;
    for row in &layout.view_grid {
        if h_remaining <= 0 {
            break;
        }
        let mut max_can_add = 0;
        for &view_key in row {
            let view = &mut view_map[view_key].view;
            let max_needed_h = view.max_needed_size(perspectives, regions).y;
            let missing_for_max_needed = max_needed_h - view.viewport_rect.h;
            let can_add = min(missing_for_max_needed, h_remaining);
            max_can_add = max(max_can_add, can_add);
            view.viewport_rect.h += can_add;
        }
        h_remaining -= max_can_add;
    }
    // Lay out
    let mut x_cursor = hex_iface_rect.x + layout.margin;
    let mut y_cursor = hex_iface_rect.y + layout.margin;
    for row in &layout.view_grid {
        let mut max_h = 0;
        for &view_key in row {
            let view = &mut view_map[view_key].view;
            view.viewport_rect.x = x_cursor;
            view.viewport_rect.y = y_cursor;
            x_cursor += view.viewport_rect.w + layout.margin;
            max_h = max(max_h, view.viewport_rect.h);
        }
        x_cursor = hex_iface_rect.x + layout.margin;
        y_cursor += max_h + layout.margin;
    }
}
