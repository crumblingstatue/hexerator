use crate::region::Region;

/// A "perspectived" (column count) view of a region
#[derive(Debug)]
pub struct Perspective {
    /// The associated region
    pub region: Region,
    /// Column count, a.k.a alignment. The proper alignment can reveal
    /// patterns to the human eye that aren't otherwise easily recognizable.
    pub cols: usize,
}
