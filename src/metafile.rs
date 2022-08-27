use serde::{Deserialize, Serialize};

use crate::app::{LayoutMap, PerspectiveMap, RegionMap, ViewMap};

#[derive(Serialize, Deserialize)]
pub struct Metafile {
    pub named_regions: RegionMap,
    pub perspectives: PerspectiveMap,
    pub view_map: ViewMap,
    pub layout_map: LayoutMap,
}
