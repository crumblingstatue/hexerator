use {super::WinCtx, crate::scripting::exec_lua};

pub struct LuaWatchWindow {
    pub name: String,
    expr: String,
    watch: bool,
}

impl Default for LuaWatchWindow {
    fn default() -> Self {
        Self {
            name: "New watch window".into(),
            expr: String::new(),
            watch: false,
        }
    }
}

impl super::Window for LuaWatchWindow {
    fn ui(
        &mut self,
        WinCtx {
            ui,
            gui,
            app,
            lua,
            font,
            ..
        }: WinCtx,
    ) {
        ui.text_edit_singleline(&mut self.name);
        ui.text_edit_singleline(&mut self.expr);
        ui.checkbox(&mut self.watch, "watch");
        if self.watch {
            match exec_lua(lua, &self.expr, app, gui, font, "", None) {
                Ok(ret) => {
                    if let Some(s) = ret {
                        ui.label(s);
                    } else {
                        ui.label("No output");
                    }
                }
                Err(e) => {
                    ui.label(e.to_string());
                }
            }
        }
    }

    fn title(&self) -> &str {
        &self.name
    }
}
