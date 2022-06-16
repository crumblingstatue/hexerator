#[derive(Clone, Copy, Debug)]
pub struct Region {
    pub begin: usize,
    pub end: usize,
}

impl Region {
    pub fn size(&self) -> usize {
        // Inclusive, so add 1 to end
        (self.end + 1) - self.begin
    }
}
