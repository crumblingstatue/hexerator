use {
    super::{
        message_dialog::{Icon, MessageDialog},
        Dialog,
    },
    crate::{
        app::App,
        damage_region::DamageRegion,
        parse_radix::{parse_offset_maybe_relative, Relativity},
        shell::{msg_fail, msg_if_fail},
        slice_ext::SliceExt,
        value_color::ColorMethod,
    },
    egui,
    egui_easy_mark_standalone::easy_mark,
    rlua::Function,
    std::time::Instant,
};

#[derive(Debug, Default)]
pub struct JumpDialog {
    string_buf: String,
    relative: bool,
    just_opened: bool,
}

impl Dialog for JumpDialog {
    fn title(&self) -> &str {
        "Jump"
    }

    fn on_open(&mut self) {
        self.just_opened = true;
    }

    fn ui(&mut self, ui: &mut egui::Ui, app: &mut App, msg: &mut MessageDialog) -> bool {
        ui.horizontal(|ui| {
            ui.label("Offset");
            let re = ui.text_edit_singleline(&mut self.string_buf);
            if self.just_opened {
                re.request_focus();
            }
        });
        self.just_opened = false;
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
                    msg_fail(&e, "Failed to parse offset", msg);
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

    fn ui(&mut self, ui: &mut egui::Ui, app: &mut App, _msg: &mut MessageDialog) -> bool {
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
    just_opened: bool,
}

impl Dialog for PatternFillDialog {
    fn title(&self) -> &str {
        "Selection pattern fill"
    }

    fn on_open(&mut self) {
        self.just_opened = true;
    }

    fn ui(&mut self, ui: &mut egui::Ui, app: &mut App, msg: &mut MessageDialog) -> bool {
        let Some(sel) = app.hex_ui.selection() else {
            ui.heading("No active selection");
            return true;
        };
        let re = ui.text_edit_singleline(&mut self.pattern_string);
        if self.just_opened {
            re.request_focus();
        }
        self.just_opened = false;
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
                    msg.open(Icon::Error, "Fill parse error", e.to_string());
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
    result_info_string: String,
    err: bool,
}

impl Dialog for LuaFillDialog {
    fn title(&self) -> &str {
        "Lua fill"
    }

    fn ui(&mut self, ui: &mut egui::Ui, app: &mut App, msg: &mut MessageDialog) -> bool {
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
            msg_if_fail(app.save(), "Failed to save", msg);
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
                let res: rlua::Result<()> = try {
                    let f = chunk.eval::<Function>()?;
                    for (i, b) in app.data[sel.begin..=sel.end].iter_mut().enumerate() {
                        *b = f.call((i, *b))?;
                    }
                    app.edit_state.dirty_region = Some(sel);
                };
                if let Err(e) = res {
                    self.result_info_string = e.to_string();
                    self.err = true;
                } else {
                    self.result_info_string =
                        format!("Script took {} ms", start_time.elapsed().as_millis());
                    self.err = false;
                }
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
        if !self.result_info_string.is_empty() {
            if self.err {
                ui.label(egui::RichText::new(&self.result_info_string).color(egui::Color32::RED));
            } else {
                ui.label(&self.result_info_string);
            }
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

    fn ui(&mut self, ui: &mut egui::Ui, app: &mut App, _msg: &mut MessageDialog) -> bool {
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
