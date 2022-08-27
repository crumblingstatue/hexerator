use serde::{Deserialize, Serialize};

use crate::app::{PerspectiveMap, RegionMap, ViewKey, ViewMap};

#[derive(Serialize, Deserialize)]
pub struct Metafile {
    pub named_regions: RegionMap,
    pub perspectives: PerspectiveMap,
    pub view_map: ViewMap,
    pub shown_views: Vec<ViewKey>,
}
