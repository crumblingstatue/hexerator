use {
    crate::{app::App, gui::Dialog, meta::region::Region},
    egui::{Button, DragValue},
    mlua::Lua,
};

pub struct TruncateDialog {
    begin: usize,
    end: usize,
}

impl TruncateDialog {
    pub fn new(data_len: usize, selection: Option<Region>) -> Self {
        let (begin, end) = match selection {
            Some(region) => (region.begin, region.end),
            None => (0, data_len.saturating_sub(1)),
        };
        Self { begin, end }
    }
}

impl Dialog for TruncateDialog {
    fn title(&self) -> &str {
        "Truncate/Extend"
    }

    fn ui(
        &mut self,
        ui: &mut egui::Ui,
        app: &mut App,
        _gui: &mut crate::gui::Gui,
        _lua: &Lua,
        _font_size: u16,
        _line_spacing: u16,
    ) -> bool {
        ui.horizontal(|ui| {
            ui.label("Begin");
            ui.add(DragValue::new(&mut self.begin).range(0..=self.end.saturating_sub(1)));
            if ui
                .add_enabled(
                    self.begin != app.edit_state.cursor,
                    Button::new("From cursor"),
                )
                .clicked()
            {
                self.begin = app.edit_state.cursor;
            }
        });
        ui.horizontal(|ui| {
            ui.label("End");
            ui.add(DragValue::new(&mut self.end));
            if ui
                .add_enabled(
                    self.end != app.edit_state.cursor,
                    Button::new("From cursor"),
                )
                .clicked()
            {
                self.end = app.edit_state.cursor;
            }
        });
        let new_len = (self.end + 1) - self.begin;
        let mut text = egui::RichText::new(format!("New length: {new_len}"));
        match new_len.cmp(&app.data.orig_data_len) {
            std::cmp::Ordering::Less => text = text.color(egui::Color32::RED),
            std::cmp::Ordering::Equal => {}
            std::cmp::Ordering::Greater => text = text.color(egui::Color32::YELLOW),
        }
        ui.label(text);
        if let Some(sel) = app.hex_ui.selection() {
            if ui
                .add_enabled(
                    !(sel.begin == self.begin && sel.end == self.end),
                    Button::new("From selection"),
                )
                .clicked()
            {
                self.begin = sel.begin;
                self.end = sel.end;
            }
        } else {
            ui.add_enabled(false, Button::new("From selection"));
        }
        ui.separator();
        let text = egui::RichText::new("⚠ Truncate/Extend ⚠").color(egui::Color32::RED);
        let mut retain = true;
        ui.horizontal(|ui| {
            if ui
                .button(text)
                .on_hover_text("This will change the length of the data")
                .clicked()
            {
                app.data.resize(self.end + 1, 0);
                app.data.drain(0..self.begin);
                app.hex_ui.select_a = None;
                app.hex_ui.select_b = None;
                app.data.dirty_region = Some(Region {
                    begin: 0,
                    end: app.data.len(),
                });
            }
            if ui.button("Close").clicked() {
                retain = false;
            }
        });
        retain
    }
}
