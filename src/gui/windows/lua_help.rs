use {
    crate::{
        app::App,
        gui::{window_open::WindowOpen, Gui},
    },
    egui_commonmark::CommonMarkViewer,
};

#[derive(Default)]
pub struct LuaHelpWindow {
    pub open: WindowOpen,
}

impl LuaHelpWindow {
    pub fn ui(ui: &mut egui::Ui, _gui: &mut Gui, app: &mut App) {
        egui::ScrollArea::vertical()
            .max_height(500.0)
            .show(ui, |ui| {
                CommonMarkViewer::new("help_cm").show(
                    ui,
                    &mut app.md_cache,
                    include_str!("../../../markdown/lua_help.md"),
                );
            });
    }
}
