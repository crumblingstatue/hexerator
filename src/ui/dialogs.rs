use egui_easy_mark_standalone::easy_mark;
use egui_sfml::egui;
use rlua::{Function, Lua};

use crate::{
    app::App,
    damage_region::DamageRegion,
    parse_radix::{parse_offset_maybe_relative, Relativity},
    shell::{msg_fail, msg_if_fail, msg_warn},
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
        easy_mark(
            ui,
            "Accepts both decimal and hexadecimal.\nPrefix with `0x` to force hex.\n\
             Prefix with `+` to add to current offset, `-` to subtract",
        );
        if ui.input().key_pressed(egui::Key::Enter) {
            match parse_offset_maybe_relative(&self.string_buf) {
                Ok((offset, relativity)) => {
                    let offset = match relativity {
                        Relativity::Absolute => offset,
                        Relativity::RelAdd => app.edit_state.cursor.saturating_add(offset),
                        Relativity::RelSub => app.edit_state.cursor.saturating_sub(offset),
                    };
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

#[derive(Debug, Default)]
pub struct LuaFillDialog;

impl Dialog for LuaFillDialog {
    fn title(&self) -> &str {
        "Lua fill"
    }

    fn ui(&mut self, ui: &mut egui::Ui, app: &mut App) -> bool {
        let Some(sel) = App::selection(&app.select_a, &app.select_b) else {
            ui.heading("No active selection");
            return true;
        };
        let ctrl_enter = ui
            .input_mut()
            .consume_key(egui::Modifiers::CTRL, egui::Key::Enter);
        let ctrl_s = ui
            .input_mut()
            .consume_key(egui::Modifiers::CTRL, egui::Key::S);
        if ctrl_s {
            msg_if_fail(app.save(), "Failed to save");
        }
        egui::TextEdit::multiline(&mut app.meta.misc.fill_lua_script)
            .code_editor()
            .desired_width(f32::INFINITY)
            .show(ui);
        if ui.button("Execute").clicked() || ctrl_enter {
            let lua = Lua::default();
            lua.context(|ctx| {
                let chunk = ctx.load(&app.meta.misc.fill_lua_script);
                match chunk.eval::<Function>() {
                    Ok(f) => {
                        let res: rlua::Result<()> = try {
                            for (i, b) in app.data[sel.begin..=sel.end].iter_mut().enumerate() {
                                *b = f.call(i)?;
                            }
                        };
                        msg_if_fail(res, "Failed to execute lua");
                        app.dirty_region = Some(sel);
                    }
                    Err(e) => msg_fail(&e, "Failed to exec lua"),
                }
            });
        }
        let close = ui.button("Close").clicked();
        if app.dirty_region.is_some() {
            ui.label(
                egui::RichText::new("Unsaved changes")
                    .italics()
                    .color(egui::Color32::YELLOW)
                    .code(),
            );
        } else {
            ui.label(
                egui::RichText::new("No unsaved changes")
                    .color(egui::Color32::GREEN)
                    .code(),
            );
        }
        easy_mark(ui, "`ctrl+enter` to execute, `ctrl+s` to save file");
        !close
    }
}
