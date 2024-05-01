use {
    crate::{
        app::App,
        gui::{message_dialog::MessageDialog, Dialog, FileOps},
        preferences::Autoreload,
    },
    egui_sfml::sfml::graphics::Font,
    mlua::Lua,
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
        _file_ops: &mut FileOps,
    ) -> bool {
        egui::ComboBox::from_label("Auto reload")
            .selected_text(app.preferences.auto_reload.label())
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut app.preferences.auto_reload,
                    Autoreload::Disabled,
                    Autoreload::Disabled.label(),
                );
                ui.selectable_value(
                    &mut app.preferences.auto_reload,
                    Autoreload::All,
                    Autoreload::All.label(),
                );
                ui.selectable_value(
                    &mut app.preferences.auto_reload,
                    Autoreload::Visible,
                    Autoreload::Visible.label(),
                );
            });
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
            || ui.input(|inp| inp.key_pressed(egui::Key::Escape))
            || ui.input(|inp| inp.key_pressed(egui::Key::Enter)))
    }
}
