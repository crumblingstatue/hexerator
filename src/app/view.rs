use crate::region::Region;

/// A view into the data
#[derive(Debug)]
pub struct View {
    /// The region this view shows
    pub region: Region,
    /// How many rows the view displays (how tall it is)
    pub rows: usize,
    /// How many columns the view displays (how wide it is)
    pub cols: usize,
}
