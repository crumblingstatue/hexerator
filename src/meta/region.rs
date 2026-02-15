use serde::{Deserialize, Serialize};

/// An inclusive region spanning `begin` to `end`
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Region {
    pub begin: usize,
    pub end: usize,
}

impl Region {
    pub fn len(&self) -> usize {
        // Inclusive, so add 1 to end
        (self.end + 1).saturating_sub(self.begin)
    }

    pub(crate) fn contains(&self, idx: usize) -> bool {
        (self.begin..=self.end).contains(&idx)
    }

    pub(crate) fn contains_region(&self, reg: &Self) -> bool {
        self.begin <= reg.begin && self.end >= reg.end
    }
    pub fn to_range(self) -> std::ops::RangeInclusive<usize> {
        self.begin..=self.end
    }
    /// The "previous chunk" with the same length
    pub fn prev_chunk(self) -> Option<Self> {
        let len = self.len();
        self.begin.checked_sub(len).map(|new_begin| Self {
            begin: new_begin,
            // INVARIANT: begin < end
            end: self.end - len,
        })
    }
    /// The "next chunk" with the same length
    pub fn next_chunk(self) -> Self {
        let len = self.len();
        Self {
            begin: self.begin + len,
            end: self.end + len,
        }
    }
}
