use {
    super::{WinCtx, WindowOpen},
    crate::{
        app::App,
        gui::Gui,
        meta::{ScriptKey, ScriptMap},
        scripting::exec_lua,
        shell::msg_if_fail,
    },
    egui_code_editor::{CodeEditor, Syntax},
    egui_sfml::sfml::graphics::Font,
    mlua::Lua,
};

#[derive(Default)]
pub struct ScriptManagerWindow {
    pub open: WindowOpen,
    selected: Option<ScriptKey>,
}

impl super::Window for ScriptManagerWindow {
    fn ui(
        &mut self,
        WinCtx {
            ui,
            gui,
            app,
            rwin: _,
            lua,
            font,
        }: WinCtx,
    ) {
        let mut scripts = std::mem::take(&mut app.meta_state.meta.scripts);
        scripts.retain(|key, script| {
            let mut retain = true;
            ui.horizontal(|ui| {
                if app.meta_state.meta.onload_script == Some(key) {
                    ui.label("⚡")
                        .on_hover_text("This script executes on document load");
                }
                if ui
                    .selectable_label(self.selected == Some(key), &script.name)
                    .clicked()
                {
                    self.selected = Some(key);
                }
                if ui.button("⚡ Execute").clicked() {
                    let result = exec_lua(lua, &script.content, app, gui, font, "", Some(key));
                    msg_if_fail(result, "Failed to execute script", &mut gui.msg_dialog);
                }
                if ui.button("Delete").clicked() {
                    retain = false;
                }
            });
            retain
        });
        ui.separator();
        self.selected_script_ui(ui, gui, app, lua, font, &mut scripts);
        std::mem::swap(&mut app.meta_state.meta.scripts, &mut scripts);
    }

    fn title(&self) -> &str {
        "Script manager"
    }
}

impl ScriptManagerWindow {
    fn selected_script_ui(
        &mut self,
        ui: &mut egui::Ui,
        gui: &mut Gui,
        app: &mut App,
        lua: &Lua,
        font: &Font,
        scripts: &mut ScriptMap,
    ) {
        let Some(key) = self.selected else {
            return;
        };
        let scr = &mut scripts[key];
        ui.label("Description");
        ui.text_edit_multiline(&mut scr.desc);
        ui.label("Code");
        egui::ScrollArea::vertical().show(ui, |ui| {
            CodeEditor::default()
                .with_syntax(Syntax::lua())
                .show(ui, &mut scr.content);
        });
        if ui.button("⚡ Execute").clicked() {
            let result = exec_lua(lua, &scr.content, app, gui, font, "", Some(key));
            msg_if_fail(result, "Failed to execute script", &mut gui.msg_dialog);
        }
        if ui.button("⚡ Set as onload script").clicked() {
            app.meta_state.meta.onload_script = Some(key);
        }
    }
}
