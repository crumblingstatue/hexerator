use {
    crate::{
        app::App,
        damage_region::DamageRegion,
        find_util,
        gui::{Dialog, message_dialog::Icon},
        slice_ext::SliceExt as _,
    },
    mlua::Lua,
};

#[derive(Debug, Default)]
pub struct PatternFillDialog {
    pattern_string: String,
    just_opened: bool,
}

impl Dialog for PatternFillDialog {
    fn title(&self) -> &str {
        "Selection pattern fill"
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
        let re = ui.add(
            egui::TextEdit::singleline(&mut self.pattern_string)
                .hint_text("Hex pattern (e.g. `00 ff 00`)"),
        );
        if self.just_opened {
            re.request_focus();
        }
        self.just_opened = false;
        if ui.input(|inp| inp.key_pressed(egui::Key::Enter)) {
            let values: Result<Vec<u8>, _> = find_util::parse_hex_string(&self.pattern_string);
            match values {
                Ok(values) => {
                    for reg in app.hex_ui.selected_regions() {
                        let range = reg.to_range();
                        let Some(data_slice) = app.data.get_mut(range.clone()) else {
                            gui.msg_dialog.open(Icon::Error, "Pattern fill error", format!("Invalid range for fill.\nRequested range: {range:?}\nData length: {}", app.data.len()));
                            return false;
                        };
                        data_slice.pattern_fill(&values);
                        app.data.widen_dirty_region(DamageRegion::RangeInclusive(range));
                    }
                    false
                }
                Err(e) => {
                    gui.msg_dialog.open(Icon::Error, "Fill parse error", e.to_string());
                    true
                }
            }
        } else {
            true
        }
    }
    fn has_close_button(&self) -> bool {
        true
    }
}
