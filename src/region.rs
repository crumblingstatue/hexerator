use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Region {
    pub begin: usize,
    pub end: usize,
}

impl Region {
    pub const fn len(&self) -> usize {
        // Inclusive, so add 1 to end
        (self.end + 1).saturating_sub(self.begin)
    }

    pub(crate) fn contains(&self, idx: usize) -> bool {
        (self.begin..=self.end).contains(&idx)
    }
}
