use serde::{Deserialize, Serialize};

use crate::app::{NamedView, PerspectiveMap, RegionMap};

#[derive(Serialize, Deserialize)]
pub struct Metafile {
    pub named_regions: RegionMap,
    pub perspectives: PerspectiveMap,
    pub views: Vec<NamedView>,
}
