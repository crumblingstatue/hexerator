use {
    super::WindowCtxt,
    crate::{
        gui::window_open::WindowOpen, meta::ScriptKey, scripting::exec_lua, shell::msg_if_fail,
    },
    std::collections::HashMap,
};

type MsgBuf = Vec<ConMsg>;
type MsgBufMap = HashMap<ScriptKey, MsgBuf>;

#[derive(Default)]
pub struct LuaConsoleWindow {
    pub open: WindowOpen,
    pub msg_bufs: MsgBufMap,
    pub eval_buf: String,
    pub active_msg_buf: Option<ScriptKey>,
    pub default_msg_buf: MsgBuf,
}

impl LuaConsoleWindow {
    fn msg_buf(&mut self) -> &mut MsgBuf {
        match self.active_msg_buf {
            Some(key) => self
                .msg_bufs
                .get_mut(&key)
                .unwrap_or(&mut self.default_msg_buf),
            None => &mut self.default_msg_buf,
        }
    }
    pub fn msg_buf_for_key(&mut self, key: Option<ScriptKey>) -> &mut MsgBuf {
        match key {
            Some(key) => self.msg_bufs.entry(key).or_default(),
            None => &mut self.default_msg_buf,
        }
    }
}

pub enum ConMsg {
    Plain(String),
    OffsetLink {
        text: String,
        offset: usize,
    },
    RangeLink {
        text: String,
        start: usize,
        end: usize,
    },
}

impl LuaConsoleWindow {
    pub fn ui(
        &mut self,
        WindowCtxt {
            ui,
            gui,
            app,
            lua,
            font,
            ..
        }: WindowCtxt,
    ) {
        ui.horizontal(|ui| {
            if ui
                .selectable_label(self.active_msg_buf.is_none(), "Default")
                .clicked()
            {
                self.active_msg_buf = None;
            }
            for k in self.msg_bufs.keys() {
                if ui
                    .selectable_label(
                        self.active_msg_buf == Some(*k),
                        &app.meta_state.meta.scripts[*k].name,
                    )
                    .clicked()
                {
                    self.active_msg_buf = Some(*k);
                }
            }
        });
        ui.separator();
        ui.horizontal(|ui| {
            let re = ui.text_edit_singleline(&mut self.eval_buf);
            if ui.button("x").on_hover_text("Clear input").clicked() {
                self.eval_buf.clear();
            }
            if ui.button("Eval").clicked()
                || (ui.input(|inp| inp.key_pressed(egui::Key::Enter)) && re.lost_focus())
            {
                let code = &self.eval_buf.clone();
                if let Err(e) = exec_lua(lua, code, app, gui, font, "", self.active_msg_buf) {
                    self.msg_buf().push(ConMsg::Plain(e.to_string()));
                }
            }
            if ui.button("Clear log").clicked() {
                self.msg_buf().clear();
            }
            if ui.button("Copy to clipboard").clicked() {
                let mut buf = String::new();
                for msg in self.msg_buf() {
                    match msg {
                        ConMsg::Plain(s) => {
                            buf.push_str(s);
                            buf.push('\n')
                        }
                        ConMsg::OffsetLink { text, offset } => {
                            buf.push_str(&format!("{offset}: {text}\n"))
                        }
                        ConMsg::RangeLink { text, start, end } => {
                            buf.push_str(&format!("{start}..={end}: {text}\n"))
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
                for msg in &*self.msg_buf() {
                    match msg {
                        ConMsg::Plain(text) => {
                            ui.label(text);
                        }
                        ConMsg::OffsetLink { text, offset } => {
                            if ui.link(text).clicked() {
                                app.search_focus(*offset);
                            }
                        }
                        ConMsg::RangeLink { text, start, end } => {
                            if ui.link(text).clicked() {
                                app.hex_ui.select_a = Some(*start);
                                app.hex_ui.select_b = Some(*end);
                                app.search_focus(*start);
                            }
                        }
                    }
                }
            });
    }
}
