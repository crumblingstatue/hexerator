use {
    crate::{
        app::App,
        gui::{window_open::WindowOpen, Gui},
        scripting::exec_lua,
        shell::msg_if_fail,
    },
    egui_sfml::sfml::graphics::Font,
    mlua::Lua,
};

#[derive(Default)]
pub struct LuaConsoleWindow {
    pub open: WindowOpen,
    pub messages: Vec<ConMsg>,
    pub eval_buf: String,
}

pub enum ConMsg {
    Plain(String),
    OffsetLink { text: String, offset: usize },
}

impl LuaConsoleWindow {
    pub fn ui(ui: &mut egui::Ui, gui: &mut Gui, app: &mut App, lua: &Lua, font: &Font) {
        ui.horizontal(|ui| {
            let re = ui.text_edit_singleline(&mut gui.lua_console_window.eval_buf);
            if ui.button("x").on_hover_text("Clear input").clicked() {
                gui.lua_console_window.eval_buf.clear();
            }
            if ui.button("Eval").clicked()
                || (ui.input(|inp| inp.key_pressed(egui::Key::Enter)) && re.lost_focus())
            {
                let code = &gui.lua_console_window.eval_buf.clone();
                if let Err(e) = exec_lua(lua, code, app, gui, font) {
                    gui.lua_console_window
                        .messages
                        .push(ConMsg::Plain(e.to_string()));
                }
            }
            if ui.button("Clear log").clicked() {
                gui.lua_console_window.messages.clear();
            }
            if ui.button("Copy to clipboard").clicked() {
                let mut buf = String::new();
                for msg in &gui.lua_console_window.messages {
                    match msg {
                        ConMsg::Plain(s) => {
                            buf.push_str(s);
                            buf.push('\n')
                        }
                        ConMsg::OffsetLink { text, offset } => {
                            buf.push_str(&format!("{offset}: {text}\n"))
                        }
                    }
                }
                msg_if_fail(
                    app.clipboard.set_text(buf),
                    "Failed to copy clipboard text",
                    &mut gui.msg_dialog,
                );
            }
        });
        ui.separator();
        egui::ScrollArea::vertical()
            .auto_shrink([false, true])
            .show(ui, |ui| {
                for msg in &gui.lua_console_window.messages {
                    match msg {
                        ConMsg::Plain(text) => {
                            ui.label(text);
                        }
                        ConMsg::OffsetLink { text, offset } => {
                            if ui.link(text).clicked() {
                                app.search_focus(*offset);
                            }
                        }
                    }
                }
            });
    }
}
