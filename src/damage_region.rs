pub enum DamageRegion {
    Single(usize),
    Range(std::ops::Range<usize>),
    RangeInclusive(std::ops::RangeInclusive<usize>),
}

impl DamageRegion {
    pub(crate) const fn begin(&self) -> usize {
        match self {
            Self::Single(offset) => *offset,
            Self::Range(range) => range.start,
            Self::RangeInclusive(range) => *range.start(),
        }
    }

    pub(crate) const fn end(&self) -> usize {
        match self {
            Self::Single(offset) => *offset,
            Self::Range(range) => range.end - 1,
            Self::RangeInclusive(range) => *range.end(),
        }
    }
}
