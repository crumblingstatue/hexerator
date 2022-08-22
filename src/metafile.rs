use serde::{Deserialize, Serialize};

use crate::app::{PerspectiveMap, RegionMap};

#[derive(Serialize, Deserialize)]
pub struct Metafile {
    pub named_regions: RegionMap,
    pub perspectives: PerspectiveMap,
}
