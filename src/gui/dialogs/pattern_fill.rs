use {
    crate::{
        app::App,
        damage_region::DamageRegion,
        gui::{message_dialog::Icon, Dialog},
        slice_ext::SliceExt,
    },
    egui_sfml::sfml::graphics::Font,
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
        _font: &Font,
    ) -> bool {
        let Some(sel) = app.hex_ui.selection() else {
            ui.heading("No active selection");
            return !ui.button("Close").clicked();
        };
        let re = ui.text_edit_singleline(&mut self.pattern_string);
        if self.just_opened {
            re.request_focus();
        }
        self.just_opened = false;
        if ui.input(|inp| inp.key_pressed(egui::Key::Enter)) {
            let values: Result<Vec<u8>, _> = parse_pattern_string(&self.pattern_string);
            match values {
                Ok(values) => {
                    let range = sel.begin..=sel.end;
                    let Some(data_slice) = app.data.get_mut(range.clone()) else {
                        gui.msg_dialog.open(Icon::Error, "Pattern fill error", format!("Invalid range for fill.\nRequested range: {range:?}\nData length: {}", app.data.len()));
                        return false;
                    };
                    data_slice.pattern_fill(&values);
                    app.edit_state
                        .widen_dirty_region(DamageRegion::RangeInclusive(range));
                    false
                }
                Err(e) => {
                    gui.msg_dialog
                        .open(Icon::Error, "Fill parse error", e.to_string());
                    true
                }
            }
        } else {
            !ui.button("Close").clicked()
        }
    }
}

pub fn parse_pattern_string(string: &str) -> Result<Vec<u8>, std::num::ParseIntError> {
    string
        .split(' ')
        .map(|token| u8::from_str_radix(token, 16))
        .collect()
}
