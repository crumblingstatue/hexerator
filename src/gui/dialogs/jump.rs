use {
    crate::{
        app::App,
        gui::Dialog,
        parse_radix::{Relativity, parse_offset_maybe_relative},
        shell::msg_fail,
    },
    mlua::Lua,
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
        gui: &mut crate::gui::Gui,
        _lua: &Lua,
        _font_size: u16,
        _line_spacing: u16,
    ) -> bool {
        ui.horizontal(|ui| {
            ui.label("Offset");
            let re = ui.text_edit_singleline(&mut self.string_buf);
            if self.just_opened {
                re.request_focus();
            }
        });
        self.just_opened = false;
        ui.label(
            "Accepts both decimal and hexadecimal.\nPrefix with `0x` to force hex.\n\
        Prefix with `+` to add to current offset, `-` to subtract",
        );
        if let Some(hard_seek) = app.src_args.hard_seek {
            ui.checkbox(&mut self.absolute, "Absolute")
                .on_hover_text("Subtract the offset from hard-seek");
            let label = format!("hard-seek is at {hard_seek} (0x{hard_seek:X})");
            ui.text_edit_multiline(&mut &label[..]);
        }
        if ui.input(|inp| inp.key_pressed(egui::Key::Enter)) {
            // Just close the dialog without error on empty text input
            if self.string_buf.trim().is_empty() {
                return false;
            }
            match parse_offset_maybe_relative(&self.string_buf) {
                Ok((offset, relativity)) => {
                    let offset = match relativity {
                        Relativity::Absolute => {
                            if let Some(hard_seek) = app.src_args.hard_seek
                                && self.absolute
                            {
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
                    msg_fail(&e, "Failed to parse offset", &mut gui.msg_dialog);
                    true
                }
            }
        } else {
            !(ui.input(|inp| inp.key_pressed(egui::Key::Escape)))
        }
    }
}
