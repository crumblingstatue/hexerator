use {
    crate::{
        app::App,
        gui::{message_dialog::MessageDialog, Dialog},
        shell::msg_if_fail,
    },
    egui,
    egui_easy_mark_standalone::easy_mark,
    egui_sfml::sfml::graphics::Font,
    rlua::{Function, Lua},
    std::time::Instant,
};

#[derive(Debug, Default)]
pub struct LuaFillDialog {
    result_info_string: String,
    err: bool,
}

impl Dialog for LuaFillDialog {
    fn title(&self) -> &str {
        "Lua fill"
    }

    fn ui(
        &mut self,
        ui: &mut egui::Ui,
        app: &mut App,
        msg: &mut MessageDialog,
        lua: &Lua,
        _font: &Font,
        _events: &mut crate::event::EventQueue,
    ) -> bool {
        let Some(sel) = app.hex_ui.selection() else {
            ui.heading("No active selection");
            return !ui.button("Close").clicked();
        };
        let ctrl_enter =
            ui.input_mut(|inp| inp.consume_key(egui::Modifiers::CTRL, egui::Key::Enter));

        let ctrl_s = ui.input_mut(|inp| inp.consume_key(egui::Modifiers::CTRL, egui::Key::S));
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
            lua.context(|ctx| {
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
