use {
    crate::{
        app::App,
        gui::{message_dialog::MessageDialog, Dialog},
        parse_radix::{parse_offset_maybe_relative, Relativity},
        shell::msg_fail,
    },
    egui,
    egui_easy_mark_standalone::easy_mark,
};

#[derive(Debug, Default)]
pub struct JumpDialog {
    string_buf: String,
    relative: bool,
    just_opened: bool,
}

impl Dialog for JumpDialog {
    fn title(&self) -> &str {
        "Jump"
    }

    fn on_open(&mut self) {
        self.just_opened = true;
    }

    fn ui(&mut self, ui: &mut egui::Ui, app: &mut App, msg: &mut MessageDialog) -> bool {
        ui.horizontal(|ui| {
            ui.label("Offset");
            let re = ui.text_edit_singleline(&mut self.string_buf);
            if self.just_opened {
                re.request_focus();
            }
        });
        self.just_opened = false;
        easy_mark(
            ui,
            "Accepts both decimal and hexadecimal.\nPrefix with `0x` to force hex.\n\
             Prefix with `+` to add to current offset, `-` to subtract",
        );
        ui.checkbox(&mut self.relative, "Relative")
            .on_hover_text("Relative to --hard-seek");
        if ui.input().key_pressed(egui::Key::Enter) {
            match parse_offset_maybe_relative(&self.string_buf) {
                Ok((offset, relativity)) => {
                    let offset = match relativity {
                        Relativity::Absolute => {
                            if let Some(hard_seek) = app.args.src.hard_seek {
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
            !(ui.input().key_pressed(egui::Key::Escape))
        }
    }
}
