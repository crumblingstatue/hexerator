use serde::{Deserialize, Serialize};

use crate::app::NamedRegion;

#[derive(Serialize, Deserialize)]
pub struct Metafile {
    pub named_regions: Vec<NamedRegion>,
}
