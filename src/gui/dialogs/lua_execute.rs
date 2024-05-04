use {
    super::pattern_fill::parse_pattern_string,
    crate::{
        app::App,
        gui::{Dialog, Gui},
        meta::{
            region::Region,
            value_type::{self, EndianedPrimitive as _, ValueType},
            Bookmark, NamedRegion,
        },
        shell::msg_if_fail,
        slice_ext::SliceExt,
    },
    egui_code_editor::{CodeEditor, Syntax},
    egui_commonmark::CommonMarkViewer,
    egui_extras::{Size, StripBuilder},
    egui_sfml::sfml::graphics::Font,
    mlua::{ExternalError, ExternalResult, Lua, UserData},
    std::time::Instant,
};

#[derive(Debug, Default)]
pub struct LuaExecuteDialog {
    result_info_string: String,
    err: bool,
}

struct LuaExecContext<'app, 'gui, 'font> {
    app: &'app mut App,
    gui: &'gui mut Gui,
    font: &'font Font,
}

impl<'app, 'gui, 'font> UserData for LuaExecContext<'app, 'gui, 'font> {
    fn add_methods<'lua, T: mlua::UserDataMethods<'lua, Self>>(methods: &mut T) {
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
                .load_file(path.into(), true, exec.font, &mut exec.gui.msg_dialog)
                .map_err(|e| e.into_lua_err())?;
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
                    .ok_or("no such bookmark".into_lua_err())?;
                bm.write_int(&mut exec.app.data[bm.offset..], val)
                    .map_err(|e| e.into_lua_err())?;
                Ok(())
            },
        );
        methods.add_method_mut(
            "region_pattern_fill",
            |_ctx, exec, (name, pattern): (String, String)| {
                let reg = exec
                    .app
                    .meta_state
                    .meta
                    .region_by_name_mut(&name)
                    .ok_or("no such region".into_lua_err())?;
                let pat = parse_pattern_string(&pattern).map_err(|e| e.into_lua_err())?;
                exec.app.data[reg.region.begin..=reg.region.end].pattern_fill(&pat);
                Ok(())
            },
        );
        methods.add_method_mut("find_result_offsets", |_ctx, exec, ()| {
            Ok(exec.gui.find_dialog.results_vec.clone())
        });
        methods.add_method_mut("read_u8", |_ctx, exec, (offset,): (usize,)| {
            match exec.app.data.get(offset) {
                Some(byte) => Ok(*byte),
                None => Err("out of bounds".into_lua_err()),
            }
        });
        methods.add_method_mut("read_u32_le", |_ctx, exec, (offset,): (usize,)| match exec
            .app
            .data
            .get(offset..offset + 4)
        {
            Some(slice) => value_type::U32Le::from_byte_slice(slice)
                .ok_or_else(|| "Failed to convert".into_lua_err()),
            None => Err("out of bounds".into_lua_err()),
        });
        methods.add_method_mut(
            "fill_range",
            |_ctx, exec, (start, end, fill): (usize, usize, u8)| match exec
                .app
                .data
                .get_mut(start..end)
            {
                Some(slice) => {
                    slice.fill(fill);
                    Ok(())
                }
                None => Err("out of bounds".into_lua_err()),
            },
        );
        methods.add_method_mut(
            "set_dirty_region",
            |_ctx, exec, (begin, end): (usize, usize)| {
                exec.app.edit_state.dirty_region = Some(Region { begin, end });
                Ok(())
            },
        );
        methods.add_method_mut("save", |_ctx, exec, ()| {
            exec.app.save(&mut exec.gui.msg_dialog).into_lua_err()?;
            Ok(())
        });
        methods.add_method_mut(
            "bookmark_offset",
            |_ctx, exec, (name,): (String,)| match exec
                .app
                .meta_state
                .meta
                .bookmark_by_name_mut(&name)
            {
                Some(bm) => Ok(bm.offset),
                None => Err(format!("no such bookmark: {name}").into_lua_err()),
            },
        );
        methods.add_method_mut(
            "add_bookmark",
            |_ctx, exec, (offset, name): (usize, String)| {
                exec.app.meta_state.meta.bookmarks.push(Bookmark {
                    offset,
                    label: name,
                    desc: String::new(),
                    value_type: ValueType::None,
                });
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
        gui: &mut crate::gui::Gui,
        lua: &Lua,
        font: &Font,
    ) -> bool {
        let mut keep_open = true;
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
                    if ui.button("Execute").clicked() || ctrl_enter {
                        let start_time = Instant::now();
                        let lua_script = app.meta_state.meta.misc.exec_lua_script.clone();
                        let result = lua.scope(|scope| {
                            let res: mlua::Result<()> = try {
                                let chunk = lua.load(&lua_script);
                                let fun = chunk.into_function()?;
                                let app = scope.create_nonstatic_userdata(LuaExecContext {
                                    app: &mut *app,
                                    gui,
                                    font,
                                })?;
                                if let Some(env) = fun.environment() {
                                    env.set("hx", app)?;
                                }
                                fun.call(())?;
                            };
                            if let Err(e) = res {
                                self.result_info_string = e.to_string();
                                self.err = true;
                            } else {
                                self.result_info_string =
                                    format!("Script took {} ms", start_time.elapsed().as_millis());
                                self.err = false;
                            }
                            Ok(())
                        });
                        msg_if_fail(result, "Lua exec error", &mut gui.msg_dialog);
                    }
                    ui.horizontal(|ui| {
                        if ui.button("Load script...").clicked() {
                            gui.fileops.load_lua_script();
                        }
                        if ui.button("Save script...").clicked() {
                            gui.fileops.save_lua_script();
                        }
                    });
                    ui.separator();
                    if ui.button("Close").clicked() {
                        keep_open = false;
                    }
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
                    CommonMarkViewer::new("viewer").show(
                        ui,
                        &mut app.md_cache,
                        "`ctrl+enter` to execute, `ctrl+s` to save file",
                    );
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
        keep_open
    }
}
