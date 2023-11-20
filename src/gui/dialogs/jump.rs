use {
    crate::{
        app::App,
        event::EventQueue,
        gui::{message_dialog::MessageDialog, Dialog},
        parse_radix::{parse_offset_maybe_relative, Relativity},
        shell::msg_fail,
    },
    egui,
    egui_commonmark::CommonMarkViewer,
    egui_sfml::sfml::graphics::Font,
    rlua::Lua,
};

#[derive(Debug, Default)]
pub struct JumpDialog {
    string_buf: String,
    absolute: bool,
    just_opened: bool,
}

impl Dialog for JumpDialog {
    fn title(&self) -> &str {
        "Jump"
    }

    fn on_open(&mut self) {
        self.just_opened = true;
    }

    fn ui(
        &mut self,
        ui: &mut egui::Ui,
        app: &mut App,
        msg: &mut MessageDialog,
        _lua: &Lua,
        _font: &Font,
        _events: &mut EventQueue,
    ) -> bool {
        ui.horizontal(|ui| {
            ui.label("Offset");
            let re = ui.text_edit_singleline(&mut self.string_buf);
            if self.just_opened {
                re.request_focus();
            }
        });
        self.just_opened = false;
        CommonMarkViewer::new("viewer").show(
            ui,
            &mut app.md_cache,
            "Accepts both decimal and hexadecimal.\nPrefix with `0x` to force hex.\n\
             Prefix with `+` to add to current offset, `-` to subtract",
        );
        if let Some(hard_seek) = app.args.src.hard_seek {
            ui.checkbox(&mut self.absolute, "Absolute")
                .on_hover_text("Subtract the offset from hard-seek");
            let label = format!("hard-seek is at {hard_seek} (0x{hard_seek:X})");
            ui.text_edit_multiline(&mut &label[..]);
        }
        if ui.input(|inp| inp.key_pressed(egui::Key::Enter)) {
            match parse_offset_maybe_relative(&self.string_buf) {
                Ok((offset, relativity)) => {
                    let offset = match relativity {
                        Relativity::Absolute => {
                            if let Some(hard_seek) = app.args.src.hard_seek && self.absolute {
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
            !(ui.input(|inp| inp.key_pressed(egui::Key::Escape)))
        }
    }
}
