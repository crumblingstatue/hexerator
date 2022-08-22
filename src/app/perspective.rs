use serde::{Deserialize, Serialize};
use slotmap::Key;

use crate::region::Region;

use super::{RegionKey, RegionMap};

/// A "perspectived" (column count) view of a region
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Perspective {
    /// The associated region
    pub region: RegionKey,
    /// Column count, a.k.a alignment. The proper alignment can reveal
    /// patterns to the human eye that aren't otherwise easily recognizable.
    pub cols: usize,
    /// Whether row order is flipped.
    ///
    /// Sometimes binary files store images or other data "upside-down".
    /// A row order flipped perspective helps view and manipulate this kind of data better.
    pub flip_row_order: bool,
}

impl Perspective {
    /// Returns the index of the last row
    pub(crate) fn last_row_idx(&self, rmap: &RegionMap) -> usize {
        rmap[self.region].region.end / self.cols
    }
    /// Returns the index of the last column
    pub(crate) fn last_col_idx(&self, rmap: &RegionMap) -> usize {
        rmap[self.region].region.end % self.cols
    }
    pub(crate) fn byte_offset_of_row_col(&self, row: usize, col: usize, rmap: &RegionMap) -> usize {
        rmap[self.region].region.begin + (row * self.cols + col)
    }
    pub(crate) fn row_col_of_byte_offset(&self, offset: usize) -> (usize, usize) {
        (offset / self.cols, offset % self.cols)
    }
    /// Whether the columns are within `cols` and the calculated offset is within the region
    pub(crate) fn row_col_within_bound(&self, row: usize, col: usize, rmap: &RegionMap) -> bool {
        col < self.cols
            && rmap[self.region]
                .region
                .contains(self.byte_offset_of_row_col(row, col, rmap))
    }
    pub(crate) fn clamp_cols(&mut self, rmap: &RegionMap) {
        self.cols = self.cols.clamp(1, rmap[self.region].region.len())
    }
    /// Returns rows spanned by `region`, and the remainder
    pub(crate) fn region_row_span(&self, region: Region) -> (usize, usize) {
        (region.len() / self.cols, region.len() % self.cols)
    }
}

impl Default for Perspective {
    fn default() -> Self {
        Self {
            region: RegionKey::null(),
            cols: 0,
            flip_row_order: false,
        }
    }
}
