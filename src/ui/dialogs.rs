use egui_sfml::egui;

use crate::{
    app::App,
    damage_region::DamageRegion,
    shell::{msg_fail, msg_warn},
    slice_ext::SliceExt,
};

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
            !(ui.input().key_pressed(egui::Key::Escape))
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

#[derive(Debug, Default)]
pub struct PatternFillDialog {
    pattern_string: String,
}

impl Dialog for PatternFillDialog {
    fn title(&self) -> &str {
        "Selection pattern fill"
    }

    fn ui(&mut self, ui: &mut egui::Ui, app: &mut App) -> bool {
        let Some(sel) = App::selection(&app.select_a, &app.select_b) else {
            ui.heading("No active selection");
            return true;
        };
        ui.text_edit_singleline(&mut self.pattern_string)
            .request_focus();
        if ui.input().key_pressed(egui::Key::Enter) {
            let values: Result<Vec<u8>, _> = self
                .pattern_string
                .split(' ')
                .map(|token| u8::from_str_radix(token, 16))
                .collect();
            match values {
                Ok(values) => {
                    let range = sel.begin..=sel.end;
                    app.data[range.clone()].pattern_fill(&values);
                    app.widen_dirty_region(DamageRegion::RangeInclusive(range));
                    false
                }
                Err(e) => {
                    msg_warn(&format!("Fill parse error: {}", e));
                    true
                }
            }
        } else {
            true
        }
    }
}
