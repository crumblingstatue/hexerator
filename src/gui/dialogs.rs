use {
    super::Dialog,
    crate::{
        app::App,
        color::ColorMethod,
        damage_region::DamageRegion,
        parse_radix::{parse_offset_maybe_relative, Relativity},
        shell::{msg_fail, msg_if_fail, msg_warn},
        slice_ext::SliceExt,
    },
    egui_easy_mark_standalone::easy_mark,
    egui_sfml::egui,
    rlua::Function,
    std::time::Instant,
};

#[derive(Debug, Default)]
pub struct JumpDialog {
    string_buf: String,
    relative: bool,
}

impl Dialog for JumpDialog {
    fn title(&self) -> &str {
        "Jump"
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
        ui.checkbox(&mut self.relative, "Relative")
            .on_hover_text("Relative to --hard-seek");
        if ui.input().key_pressed(egui::Key::Enter) {
            match parse_offset_maybe_relative(&self.string_buf) {
                Ok((offset, relativity)) => {
                    let offset = match relativity {
                        Relativity::Absolute => {
                            if let Some(hard_seek) = app.args.src.hard_seek {
                                offset.saturating_sub(hard_seek)
                            } else {
                                offset
                            }
                        }
                        Relativity::RelAdd => app.edit_state.cursor.saturating_add(offset),
                        Relativity::RelSub => app.edit_state.cursor.saturating_sub(offset),
                    };
                    app.edit_state.cursor = offset;
                    app.center_view_on_offset(offset);
                    app.hex_ui.flash_cursor();
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

#[derive(Debug, Default)]
pub struct PatternFillDialog {
    pattern_string: String,
}

impl Dialog for PatternFillDialog {
    fn title(&self) -> &str {
        "Selection pattern fill"
    }

    fn ui(&mut self, ui: &mut egui::Ui, app: &mut App) -> bool {
        let Some(sel) = app.hex_ui.selection() else {
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
                    app.edit_state
                        .widen_dirty_region(DamageRegion::RangeInclusive(range));
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
pub struct LuaFillDialog {
    exec_time_string: String,
}

impl Dialog for LuaFillDialog {
    fn title(&self) -> &str {
        "Lua fill"
    }

    fn ui(&mut self, ui: &mut egui::Ui, app: &mut App) -> bool {
        let Some(sel) = app.hex_ui.selection() else {
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
        egui::ScrollArea::vertical()
            // 100.0 is an estimation of ui size below.
            // If we don't subtract that, the text edit tries to expand
            // beyond window height
            .max_height(ui.available_height() - 100.0)
            .show(ui, |ui| {
                egui::TextEdit::multiline(&mut app.meta_state.meta.misc.fill_lua_script)
                    .code_editor()
                    .desired_width(f32::INFINITY)
                    .show(ui);
            });
        if ui.button("Execute").clicked() || ctrl_enter {
            let start_time = Instant::now();
            app.lua.context(|ctx| {
                let chunk = ctx.load(&app.meta_state.meta.misc.fill_lua_script);
                match chunk.eval::<Function>() {
                    Ok(f) => {
                        let res: rlua::Result<()> = try {
                            for (i, b) in app.data[sel.begin..=sel.end].iter_mut().enumerate() {
                                *b = f.call((i, *b))?;
                            }
                        };
                        msg_if_fail(res, "Failed to execute lua");
                        app.edit_state.dirty_region = Some(sel);
                    }
                    Err(e) => msg_fail(&e, "Failed to exec lua"),
                }
                self.exec_time_string =
                    format!("Script took {} ms", start_time.elapsed().as_millis());
            });
        }
        let close = ui.button("Close").clicked();
        if app.edit_state.dirty_region.is_some() {
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
        if !self.exec_time_string.is_empty() {
            ui.label(&self.exec_time_string);
        }
        !close
    }
}

pub struct LuaColorDialog {
    script: String,
    err_string: String,
    auto_exec: bool,
}

impl Default for LuaColorDialog {
    fn default() -> Self {
        const DEFAULT_SCRIPT: &str =
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/lua/color.lua"));
        Self {
            script: DEFAULT_SCRIPT.into(),
            err_string: String::new(),
            auto_exec: Default::default(),
        }
    }
}

impl Dialog for LuaColorDialog {
    fn title(&self) -> &str {
        "Lua color"
    }

    fn ui(&mut self, ui: &mut egui::Ui, app: &mut App) -> bool {
        let color_data = match app.hex_ui.focused_view {
            Some(view_key) => {
                let view = &mut app.meta_state.meta.views[view_key].view;
                match &mut view.presentation.color_method {
                    ColorMethod::Custom(color_data) => &mut color_data.0,
                    _ => {
                        ui.label("Please select \"Custom\" as color scheme for the current view");
                        return !ui.button("Close").clicked();
                    }
                }
            }
            None => {
                ui.label("No active view");
                return !ui.button("Close").clicked();
            }
        };
        egui::TextEdit::multiline(&mut self.script)
            .code_editor()
            .desired_width(f32::INFINITY)
            .show(ui);
        if ui.button("Execute").clicked() || self.auto_exec {
            app.lua.context(|ctx| {
                let chunk = ctx.load(&self.script);
                let res: rlua::Result<()> = try {
                    let fun = chunk.eval::<Function>()?;
                    for (i, c) in color_data.iter_mut().enumerate() {
                        let rgb: [u8; 3] = fun.call((i,))?;
                        *c = rgb;
                    }
                };
                if let Err(e) = res {
                    self.err_string = e.to_string();
                } else {
                    self.err_string.clear();
                }
            });
        }
        ui.checkbox(&mut self.auto_exec, "Auto execute");
        if !self.err_string.is_empty() {
            ui.label(egui::RichText::new(&self.err_string).color(egui::Color32::RED));
        }
        true
    }
}
