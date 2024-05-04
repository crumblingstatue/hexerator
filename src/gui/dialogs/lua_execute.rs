use {
    crate::{
        app::App,
        gui::{Dialog, Gui},
        meta::Script,
        shell::msg_if_fail,
    },
    egui::TextBuffer,
    egui_code_editor::{CodeEditor, Syntax},
    egui_extras::{Size, StripBuilder},
    egui_sfml::sfml::graphics::Font,
    mlua::Lua,
    std::time::Instant,
};

#[derive(Debug, Default)]
pub struct LuaExecuteDialog {
    result_info_string: String,
    err: bool,
    new_script_name: String,
}

impl Dialog for LuaExecuteDialog {
    fn title(&self) -> &str {
        "Execute Lua"
    }

    fn ui(
        &mut self,
        ui: &mut egui::Ui,
        app: &mut App,
        gui: &mut crate::gui::Gui,
        lua: &Lua,
        font: &Font,
    ) -> bool {
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
        StripBuilder::new(ui)
            .size(Size::remainder())
            .size(Size::exact(300.0))
            .vertical(|mut strip| {
                strip.cell(|ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        CodeEditor::default()
                            .with_syntax(Syntax::lua())
                            .show(ui, &mut app.meta_state.meta.misc.exec_lua_script);
                    });
                });
                strip.cell(|ui| {
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui
                            .button("⚡ Execute")
                            .on_hover_text("Ctrl+Enter")
                            .clicked()
                            || ctrl_enter
                        {
                            self.exec_lua(app, lua, gui, font);
                        }
                        if ui.button("🖴 Load from file...").clicked() {
                            gui.fileops.load_lua_script();
                        }
                        if ui.button("💾 Save to file...").clicked() {
                            gui.fileops.save_lua_script();
                        }
                        if ui.button("？ Help").clicked() {
                            gui.lua_help_window.open.toggle()
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.new_script_name)
                                .hint_text("New script name"),
                        );
                        if ui.button("Add named script").clicked() {
                            app.meta_state.meta.scripts.insert(Script {
                                name: self.new_script_name.take(),
                                desc: String::new(),
                                content: app.meta_state.meta.misc.exec_lua_script.clone(),
                            });
                        }
                    });
                    ui.separator();
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
                    if !self.result_info_string.is_empty() {
                        if self.err {
                            ui.label(
                                egui::RichText::new(&self.result_info_string)
                                    .color(egui::Color32::RED),
                            );
                        } else {
                            ui.label(&self.result_info_string);
                        }
                    }
                });
            });
        true
    }
    fn has_close_button(&self) -> bool {
        true
    }
}

impl LuaExecuteDialog {
    fn exec_lua(&mut self, app: &mut App, lua: &Lua, gui: &mut Gui, font: &Font) {
        let start_time = Instant::now();
        let lua_script = app.meta_state.meta.misc.exec_lua_script.clone();
        let result = crate::scripting::exec_lua(lua, &lua_script, app, gui, font);
        if let Err(e) = result {
            self.result_info_string = e.to_string();
            self.err = true;
        } else {
            self.result_info_string =
                format!("Script took {} ms", start_time.elapsed().as_millis());
            self.err = false;
        }
    }
}
