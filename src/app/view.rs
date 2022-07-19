use crate::region::Region;

/// A view into the data
#[derive(Debug)]
pub struct View {
    /// The region this view shows
    pub region: Region,
    /// How many columns the view displays (how wide it is)
    pub cols: usize,
}
