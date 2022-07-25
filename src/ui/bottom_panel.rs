use egui_sfml::egui::{self, DragValue, TextEdit, Ui};

use crate::app::{interact_mode::InteractMode, App};

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
                ui.add(DragValue::new(&mut app.perspective.region.begin));
                ui.label("columns");
                ui.add(DragValue::new(&mut app.perspective.cols));
                let data_len = app.data.len();
                if data_len != 0 {
                    if let Some(idx) = app.focused_view {
                        let offsets = app.views[idx].offsets(&app.perspective);
                        #[expect(
                            clippy::cast_precision_loss,
                            reason = "Precision is good until 52 bits (more than reasonable)"
                        )]
                        ui.label(format!(
                            "view offset: row {} col {} byte {} ({:.2}%)",
                            offsets.row,
                            offsets.col,
                            offsets.byte,
                            (offsets.byte as f64 / data_len as f64) * 100.0
                        ));
                    }
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
    });
}
