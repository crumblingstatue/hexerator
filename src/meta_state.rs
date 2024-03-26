use {
    crate::meta::Meta,
    std::{cell::Cell, path::PathBuf, time::Instant},
};

pub struct MetaState {
    pub last_meta_backup: Cell<Instant>,
    pub current_meta_path: PathBuf,
    /// Clean copy of the metadata from last load/save
    pub clean_meta: Meta,
    pub meta: Meta,
}

impl Default for MetaState {
    fn default() -> Self {
        Self {
            meta: Meta::default(),
            clean_meta: Meta::default(),
            last_meta_backup: Cell::new(Instant::now()),
            current_meta_path: PathBuf::new(),
        }
    }
}
