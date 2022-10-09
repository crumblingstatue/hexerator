use {
    crate::{
        app::App,
        gui::{message_dialog::MessageDialog, Dialog},
    },
    egui,
    egui_sfml::sfml::graphics::Font,
    rlua::Lua,
};

#[derive(Debug)]
pub struct AutoSaveReloadDialog;

impl Dialog for AutoSaveReloadDialog {
    fn title(&self) -> &str {
        "Auto save/reload"
    }

    fn ui(
        &mut self,
        ui: &mut egui::Ui,
        app: &mut App,
        _msg: &mut MessageDialog,
        _lua: &Lua,
        _font: &Font,
    ) -> bool {
        ui.checkbox(&mut app.preferences.auto_reload, "Auto reload");
        ui.horizontal(|ui| {
            ui.label("Interval (ms)");
            ui.add(egui::DragValue::new(
                &mut app.preferences.auto_reload_interval_ms,
            ));
        });
        ui.separator();
        ui.checkbox(&mut app.preferences.auto_save, "Auto save")
            .on_hover_text("Save every time an editing action is finished");
        ui.separator();
        !(ui.button("Close (enter/esc)").clicked()
            || ui.input().key_pressed(egui::Key::Escape)
            || ui.input().key_pressed(egui::Key::Enter))
    }
}
