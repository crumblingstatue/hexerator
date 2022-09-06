use std::{cell::Cell, path::PathBuf, time::Instant};

use crate::meta::Meta;

pub struct MetaState {
    pub last_meta_backup: Cell<Instant>,
    pub current_meta_path: PathBuf,
    /// Clean copy of the metadata from last load/save
    pub clean_meta: Meta,
    pub meta: Meta,
    /// Whether metafile needs saving
    pub meta_dirty: bool,
}

impl Default for MetaState {
    fn default() -> Self {
        Self {
            meta: Meta::default(),
            clean_meta: Meta::default(),
            last_meta_backup: Cell::new(Instant::now()),
            current_meta_path: PathBuf::new(),
            meta_dirty: false,
        }
    }
}
