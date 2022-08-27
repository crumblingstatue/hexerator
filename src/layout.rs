use serde::{Deserialize, Serialize};

use crate::app::ViewKey;

/// A view layout grid for laying out views.
#[derive(Clone, Serialize, Deserialize)]
pub struct Layout {
    pub name: String,
    pub view_keys: Vec<Vec<ViewKey>>,
}

impl Layout {
    /// Iterate through all view keys
    pub fn iter(&self) -> impl Iterator<Item = ViewKey> + '_ {
        self.view_keys.iter().flatten().cloned()
    }
}
