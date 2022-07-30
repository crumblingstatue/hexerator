#[derive(Debug)]
pub struct Layout {
    pub top_gap: i16,
    pub bottom_gap: i16,
}

impl Layout {
    pub fn new() -> Self {
        Self {
            top_gap: 46,
            bottom_gap: 25,
        }
    }
}
