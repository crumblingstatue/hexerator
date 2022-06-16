#[derive(Default, Debug)]
pub struct FindDialog {
    pub open: bool,
    pub input: String,
    pub result_offsets: Vec<usize>,
    /// Used to keep track of previous/next result to go to
    pub result_cursor: usize,
    /// When Some, the results list should be scrolled to the offset of that result
    pub scroll_to: Option<usize>,
}
