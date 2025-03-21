use {
    crate::{app::App, meta::PerspectiveKey},
    hexerator_plugin_api::{HexeratorHandle, PerspectiveHandle, Plugin, PluginMethod},
    slotmap::{Key as _, KeyData},
    std::path::PathBuf,
};

pub struct PluginContainer {
    pub path: PathBuf,
    pub plugin: Box<dyn Plugin>,
    pub methods: Vec<PluginMethod>,
    // Safety: Must be last, fields are dropped in decl order.
    pub _lib: libloading::Library,
}

impl HexeratorHandle for App {
    fn debug_log(&self, msg: &str) {
        gamedebug_core::per!("{msg}");
    }

    fn get_data(&self, start: usize, end: usize) -> Option<&[u8]> {
        self.data.get(start..=end)
    }

    fn get_data_mut(&mut self, start: usize, end: usize) -> Option<&mut [u8]> {
        self.data.get_mut(start..=end)
    }

    fn selection_range(&self) -> Option<[usize; 2]> {
        self.hex_ui.selection().map(|sel| [sel.begin, sel.end])
    }

    fn perspective(&self, name: &str) -> Option<PerspectiveHandle> {
        let key = self
            .meta_state
            .meta
            .low
            .perspectives
            .iter()
            .find_map(|(k, per)| (per.name == name).then_some(k))?;
        Some(PerspectiveHandle {
            key_data: key.data().as_ffi(),
        })
    }

    fn perspective_rows(&self, ph: &PerspectiveHandle) -> Vec<&[u8]> {
        let key: PerspectiveKey = KeyData::from_ffi(ph.key_data).into();
        let per = &self.meta_state.meta.low.perspectives[key];
        let regs = &self.meta_state.meta.low.regions;
        let mut out = Vec::new();
        let n_rows = per.n_rows(regs);
        for row_idx in 0..n_rows {
            let begin = per.byte_offset_of_row_col(row_idx, 0, regs);
            out.push(&self.data[begin..begin + per.cols]);
        }
        out
    }
}

impl PluginContainer {
    pub unsafe fn new(path: PathBuf) -> anyhow::Result<Self> {
        // Safety: This will cause UB on a bad plugin. Nothing we can do.
        //
        // It's up to the user not to load bad plugins.
        unsafe {
            let lib = libloading::Library::new(&path)?;
            let plugin_init = lib.get::<fn() -> Box<dyn Plugin>>(b"hexerator_plugin_new")?;
            let plugin = plugin_init();
            Ok(Self {
                path,
                methods: plugin.methods(),
                plugin,
                _lib: lib,
            })
        }
    }
}
