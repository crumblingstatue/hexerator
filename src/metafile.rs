use serde::{Deserialize, Serialize};

use crate::app::RegionMap;

#[derive(Serialize, Deserialize)]
pub struct Metafile {
    pub named_regions: RegionMap,
}
