use egui_sfml::egui;

use crate::{app::App, msg_fail};

use super::Dialog;

#[derive(Debug, Default)]
pub struct SetCursorDialog {
    string_buf: String,
}
impl Dialog for SetCursorDialog {
    fn title(&self) -> &str {
        "Set cursor"
    }

    fn ui(&mut self, ui: &mut egui::Ui, app: &mut App) -> bool {
        ui.horizontal(|ui| {
            ui.label("Offset");
            ui.text_edit_singleline(&mut self.string_buf)
                .request_focus();
        });
        if ui.input().key_pressed(egui::Key::Enter) {
            match self.string_buf.parse::<usize>() {
                Ok(offset) => {
                    app.edit_state.cursor = offset;
                    app.center_view_on_offset(offset);
                    app.flash_cursor();
                    false
                }
                Err(e) => {
                    msg_fail(&e, "Failed to parse offset");
                    true
                }
            }
        } else {
            true
        }
    }
}

#[derive(Debug)]
pub struct AutoSaveReloadDialog;

impl Dialog for AutoSaveReloadDialog {
    fn title(&self) -> &str {
        "Auto save/reload"
    }

    fn ui(&mut self, ui: &mut egui::Ui, app: &mut App) -> bool {
        ui.checkbox(&mut app.auto_reload, "Auto reload");
        ui.horizontal(|ui| {
            ui.label("Interval (ms)");
            ui.add(egui::DragValue::new(&mut app.auto_reload_interval_ms));
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
