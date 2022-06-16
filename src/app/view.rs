/// A view into the data
#[derive(Debug)]
pub struct View {
    /// The starting offset where the view starts from
    pub start_offset: usize,
    /// How many rows the view displays (how tall it is)
    pub rows: usize,
    /// How many columns the view displays (how wide it is)
    pub cols: usize,
}
