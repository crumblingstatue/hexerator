use {
    crate::{app::App, gui::Dialog, shell::msg_if_fail},
    egui_code_editor::{CodeEditor, Syntax},
    mlua::{Function, Lua},
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
        gui: &mut crate::gui::Gui,
        lua: &Lua,
        _font_size: u16,
        _line_spacing: u16,
    ) -> bool {
        let Some(sel) = app.hex_ui.selection() else {
            ui.heading("No active selection");
            return !ui.button("Close").clicked();
        };
        let ctrl_enter =
            ui.input_mut(|inp| inp.consume_key(egui::Modifiers::CTRL, egui::Key::Enter));

        let ctrl_s = ui.input_mut(|inp| inp.consume_key(egui::Modifiers::CTRL, egui::Key::S));
        if ctrl_s {
            msg_if_fail(
                app.save(&mut gui.msg_dialog),
                "Failed to save",
                &mut gui.msg_dialog,
            );
        }
        egui::ScrollArea::vertical()
            // 100.0 is an estimation of ui size below.
            // If we don't subtract that, the text edit tries to expand
            // beyond window height
            .max_height(ui.available_height() - 100.0)
            .show(ui, |ui| {
                CodeEditor::default()
                    .with_syntax(Syntax::lua())
                    .show(ui, &mut app.meta_state.meta.misc.fill_lua_script);
            });
        if ui.button("Execute").clicked() || ctrl_enter {
            let start_time = Instant::now();
            let chunk = lua.load(&app.meta_state.meta.misc.fill_lua_script);
            let res: mlua::Result<()> = try {
                let f = chunk.eval::<Function>()?;
                for (i, b) in app.data[sel.begin..=sel.end].iter_mut().enumerate() {
                    *b = f.call((i, *b))?;
                }
                app.data.dirty_region = Some(sel);
            };
            if let Err(e) = res {
                self.result_info_string = e.to_string();
                self.err = true;
            } else {
                self.result_info_string =
                    format!("Script took {} ms", start_time.elapsed().as_millis());
                self.err = false;
            }
        }
        if app.data.dirty_region.is_some() {
            ui.label(
                egui::RichText::new("Unsaved changes")
                    .italics()
                    .color(egui::Color32::YELLOW)
                    .code(),
            );
        } else {
            ui.label(egui::RichText::new("No unsaved changes").color(egui::Color32::GREEN).code());
        }
        ui.label("ctrl+enter to execute, ctrl+s to save file");
        if !self.result_info_string.is_empty() {
            if self.err {
                ui.label(egui::RichText::new(&self.result_info_string).color(egui::Color32::RED));
            } else {
                ui.label(&self.result_info_string);
            }
        }
        true
    }
    fn has_close_button(&self) -> bool {
        true
    }
}
