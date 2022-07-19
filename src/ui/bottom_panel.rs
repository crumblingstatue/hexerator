use egui_sfml::egui::{self, Button, DragValue, Layout, TextEdit, Ui};

use crate::{
    app::{interact_mode::InteractMode, App},
    msg_if_fail,
};

pub fn ui(ui: &mut Ui, app: &mut App) {
    ui.horizontal(|ui| {
        if ui
            .selectable_label(app.interact_mode == InteractMode::View, "View (F1)")
            .clicked()
        {
            app.interact_mode = InteractMode::View;
        }
        if ui
            .selectable_label(app.interact_mode == InteractMode::Edit, "Edit (F2)")
            .clicked()
        {
            app.interact_mode = InteractMode::Edit;
        }
        ui.separator();
        match app.interact_mode {
            InteractMode::View => {
                ui.label("offset");
                ui.add(DragValue::new(&mut app.view.region.begin));
                ui.label("columns");
                ui.add(DragValue::new(&mut app.view.cols));
                let data_len = app.data.len();
                if data_len != 0 {
                    let re = ui.add(
                        TextEdit::singleline(&mut app.ui.center_offset_input)
                            .hint_text("Center view on offset"),
                    );
                    if re.lost_focus() && ui.input().key_pressed(egui::Key::Enter) {
                        if let Ok(offset) = app.ui.center_offset_input.parse() {
                            app.center_view_on_offset(offset);
                        }
                    }
                }
            }
            InteractMode::Edit => 'edit: {
                if app.data.is_empty() {
                    break 'edit;
                }
                ui.label(format!("cursor: {}", app.edit_state.cursor));
                ui.separator();
            }
        }
        ui.with_layout(Layout::right_to_left(), |ui| {
            if ui
                .checkbox(&mut gamedebug_core::enabled(), "debug (F12)")
                .clicked()
            {
                gamedebug_core::toggle();
            }
            ui.separator();
            if ui.add(Button::new("Reload (ctrl+R)")).clicked() {
                msg_if_fail(app.reload(), "Failed to reload");
            }
            if ui
                .add_enabled(
                    !app.args.read_only && app.dirty_region.is_some(),
                    Button::new("Save (ctrl+S)"),
                )
                .clicked()
            {
                msg_if_fail(app.save(), "Failed to save");
            }
            ui.separator();
            if ui.button("Restore").clicked() {
                msg_if_fail(app.restore_backup(), "Failed to restore backup");
            }
            if ui.button("Backup").clicked() {
                msg_if_fail(app.create_backup(), "Failed to create backup");
            }
        })
    });
}
