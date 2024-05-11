use {
    crate::app::App,
    hexerator_plugin_api::{HexeratorHandle, Plugin, PluginMethod},
    std::{self, path::PathBuf},
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

    fn selection_range(&self) -> Option<(usize, usize)> {
        self.hex_ui.selection().map(|sel| (sel.begin, sel.end))
    }
}

impl PluginContainer {
    pub unsafe fn new(path: PathBuf) -> anyhow::Result<Self> {
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
