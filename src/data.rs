use {
    crate::{damage_region::DamageRegion, meta::region::Region},
    std::ops::{Deref, DerefMut},
};

/// The data we are viewing/editing
#[derive(Default, Debug)]
pub struct Data {
    data: Option<DataProvider>,
    /// The region that was changed compared to the source
    pub dirty_region: Option<Region>,
    /// Original data length. Compared with current data length to detect truncation.
    pub orig_data_len: usize,
}

enum DataProvider {
    Vec(Vec<u8>),
    MmapMut(memmap2::MmapMut),
    MmapImmut(memmap2::Mmap),
}

impl std::fmt::Debug for DataProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Vec(..) => f.write_str("Vec"),
            Self::MmapMut(..) => f.write_str("MmapMut"),
            Self::MmapImmut(..) => f.write_str("MmapImmut"),
        }
    }
}

impl Data {
    pub(crate) fn clean_from_buf(buf: Vec<u8>) -> Self {
        Self {
            orig_data_len: buf.len(),
            data: Some(DataProvider::Vec(buf)),
            dirty_region: None,
        }
    }
    pub(crate) fn new_mmap_mut(mmap: memmap2::MmapMut) -> Self {
        Self {
            orig_data_len: mmap.len(),
            data: Some(DataProvider::MmapMut(mmap)),
            dirty_region: None,
        }
    }
    pub(crate) fn new_mmap_immut(mmap: memmap2::Mmap) -> Self {
        Self {
            orig_data_len: mmap.len(),
            data: Some(DataProvider::MmapImmut(mmap)),
            dirty_region: None,
        }
    }
    /// Drop any expensive allocations and reset to "empty" state
    pub(crate) fn close(&mut self) {
        self.data = None;
        self.dirty_region = None;
    }
    pub(crate) fn widen_dirty_region(&mut self, damage: DamageRegion) {
        match &mut self.dirty_region {
            Some(dirty_region) => {
                if damage.begin() < dirty_region.begin {
                    dirty_region.begin = damage.begin();
                }
                if damage.begin() > dirty_region.end {
                    dirty_region.end = damage.begin();
                }
                let end = damage.end();
                {
                    if end < dirty_region.begin {
                        gamedebug_core::per!("TODO: logic error in widen_dirty_region");
                        return;
                    }
                    if end > dirty_region.end {
                        dirty_region.end = end;
                    }
                }
            }
            None => {
                self.dirty_region = Some(Region {
                    begin: damage.begin(),
                    end: damage.end(),
                });
            }
        }
    }
    /// Clears the dirty region (asserts data is same as source), and sets length same as source
    pub(crate) fn undirty(&mut self) {
        self.dirty_region = None;
        self.orig_data_len = self.len();
    }

    pub(crate) fn resize(&mut self, new_len: usize, value: u8) {
        match &mut self.data {
            Some(DataProvider::Vec(v)) => v.resize(new_len, value),
            etc => {
                eprintln!("Data::resize: Unimplemented for {etc:?}");
            }
        }
    }

    pub(crate) fn extend_from_slice(&mut self, slice: &[u8]) {
        match &mut self.data {
            Some(DataProvider::Vec(v)) => v.extend_from_slice(slice),
            etc => {
                eprintln!("Data::extend_from_slice: Unimplemented for {etc:?}");
            }
        }
    }

    pub(crate) fn drain(&mut self, range: std::ops::Range<usize>) {
        match &mut self.data {
            Some(DataProvider::Vec(v)) => {
                v.drain(range);
            }
            etc => {
                eprintln!("Data::drain: Unimplemented for {etc:?}");
            }
        }
    }

    pub(crate) fn zero_fill_region(&mut self, region: Region) {
        let range = region.begin..=region.end;
        if let Some(data) = self.get_mut(range.clone()) {
            data.fill(0);
            self.widen_dirty_region(DamageRegion::RangeInclusive(range));
        }
    }

    pub(crate) fn reload_from_file(
        &mut self,
        src_args: &crate::args::SourceArgs,
        file: &mut std::fs::File,
    ) -> anyhow::Result<()> {
        match &mut self.data {
            Some(DataProvider::Vec(buf)) => {
                *buf = crate::app::read_contents(src_args, file)?;
            }
            etc => anyhow::bail!("Reload not supported for {etc:?}"),
        }
        self.dirty_region = None;
        Ok(())
    }

    pub(crate) fn mod_range(
        &mut self,
        range: std::ops::RangeInclusive<usize>,
        mut f: impl FnMut(&mut u8),
    ) {
        for byte in &mut self[range.clone()] {
            f(byte);
        }
        self.widen_dirty_region(range.into());
    }
}

impl Deref for Data {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        match &self.data {
            Some(DataProvider::Vec(v)) => v,
            Some(DataProvider::MmapMut(map)) => map,
            Some(DataProvider::MmapImmut(map)) => map,
            None => &[],
        }
    }
}

impl DerefMut for Data {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match &mut self.data {
            Some(DataProvider::Vec(v)) => v,
            Some(DataProvider::MmapMut(map)) => map,
            Some(DataProvider::MmapImmut(_)) => &mut [],
            None => &mut [],
        }
    }
}
