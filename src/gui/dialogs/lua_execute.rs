use {
    crate::{
        app::App,
        event::EventQueue,
        gui::{message_dialog::MessageDialog, Dialog},
        meta::{region::Region, NamedRegion},
        shell::msg_if_fail,
    },
    egui,
    egui_easy_mark_standalone::easy_mark,
    egui_sfml::sfml::graphics::Font,
    rlua::{ExternalError, Function, Lua, UserData},
    std::time::Instant,
};

#[derive(Debug, Default)]
pub struct LuaExecuteDialog {
    result_info_string: String,
    err: bool,
}

struct LuaExecContext<'app, 'msg, 'font, 'events> {
    app: &'app mut App,
    msg: &'msg mut MessageDialog,
    font: &'font Font,
    events: &'events mut EventQueue,
}

impl<'app, 'msg, 'font, 'events> UserData for LuaExecContext<'app, 'msg, 'font, 'events> {
    fn add_methods<'lua, T: rlua::UserDataMethods<'lua, Self>>(methods: &mut T) {
        methods.add_method_mut(
            "add_region",
            |_ctx, exec, (name, begin, end): (String, usize, usize)| {
                exec.app.meta_state.meta.low.regions.insert(NamedRegion {
                    name,
                    desc: String::new(),
                    region: Region { begin, end },
                });
                Ok(())
            },
        );
        methods.add_method_mut("load_file", |_ctx, exec, (path,): (String,)| {
            exec.app
                .load_file(path.into(), true, exec.font, exec.msg, exec.events)
                .map_err(|e| e.to_lua_err())?;
            Ok(())
        });
        methods.add_method_mut(
            "bookmark_set_int",
            |_ctx, exec, (name, val): (String, i64)| {
                let bm = exec
                    .app
                    .meta_state
                    .meta
                    .bookmark_by_name_mut(&name)
                    .ok_or("no such bookmark".to_lua_err())?;
                bm.write_int(&mut exec.app.data[bm.offset..], val)
                    .map_err(|e| e.to_lua_err())?;
                Ok(())
            },
        );
    }
}

impl Dialog for LuaExecuteDialog {
    fn title(&self) -> &str {
        "Execute Lua"
    }

    fn ui(
        &mut self,
        ui: &mut egui::Ui,
        app: &mut App,
        msg: &mut MessageDialog,
        lua: &Lua,
        font: &Font,
        events: &mut EventQueue,
    ) -> bool {
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
                egui::TextEdit::multiline(&mut app.meta_state.meta.misc.exec_lua_script)
                    .code_editor()
                    .desired_width(f32::INFINITY)
                    .show(ui);
            });
        if ui.button("Execute").clicked() || ctrl_enter {
            let start_time = Instant::now();
            let lua_script = app.meta_state.meta.misc.exec_lua_script.clone();
            lua.context(|ctx| {
                ctx.scope(|scope| {
                    let res: rlua::Result<()> = try {
                        /*let add_region = scope.create_function_mut(
                            ,
                        )?;
                        ctx.globals().set("add_region", add_region)?;*/
                        let chunk = ctx.load(&lua_script);
                        let f = chunk.eval::<Function>()?;
                        let app = scope.create_nonstatic_userdata(LuaExecContext {
                            app: &mut *app,
                            msg,
                            font,
                            events: &mut *events,
                        })?;
                        f.call(app)?;
                        //chunk.exec()?;
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
            });
        }
        ui.horizontal(|ui| {
            if ui.button("Load script...").clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_file() {
                    let res: anyhow::Result<()> = try {
                        app.meta_state.meta.misc.exec_lua_script = std::fs::read_to_string(path)?;
                    };
                    msg_if_fail(res, "Failed to load script", msg);
                }
            }
            if ui.button("Save script...").clicked() {
                if let Some(path) = rfd::FileDialog::new().save_file() {
                    msg_if_fail(
                        std::fs::write(path, &app.meta_state.meta.misc.exec_lua_script),
                        "Failed to save script",
                        msg,
                    );
                }
            }
        });
        ui.separator();
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
