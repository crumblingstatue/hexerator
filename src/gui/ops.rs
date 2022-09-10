//! Various common operations that are triggered by gui interactions

use crate::{meta::region::Region, meta_state::MetaState};

use super::regions_window::RegionsWindow;

pub fn add_region_from_selection(
    selection: Region,
    app_meta_state: &mut MetaState,
    gui_regions_window: &mut RegionsWindow,
) {
    let key = app_meta_state.meta.add_region_from_selection(selection);
    gui_regions_window.open.set(true);
    gui_regions_window.selected_key = Some(key);
}