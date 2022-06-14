pub enum DamageRegion {
    Single(usize),
    Range(std::ops::Range<usize>),
    RangeInclusive(std::ops::RangeInclusive<usize>),
}

impl DamageRegion {
    pub(crate) fn begin(&self) -> usize {
        match self {
            DamageRegion::Single(offset) => *offset,
            DamageRegion::Range(range) => range.start,
            DamageRegion::RangeInclusive(range) => *range.start(),
        }
    }

    pub(crate) fn end(&self) -> usize {
        match self {
            DamageRegion::Single(offset) => *offset,
            DamageRegion::Range(range) => range.end - 1,
            DamageRegion::RangeInclusive(range) => *range.end(),
        }
    }
}
