//! Various egui utility functions

use egui_sfml::egui::{pos2, text::LayoutJob, Response, TextStyle, Ui, Widget};

pub struct ButtonWithShortcut(pub &'static str, pub &'static str);

impl Widget for ButtonWithShortcut {
    fn ui(self, ui: &mut Ui) -> Response {
        button_with_shortcut(ui, self.0, self.1)
    }
}

pub fn button_with_shortcut(ui: &mut Ui, label: &str, shortcut: &str) -> Response {
    let btn_re = ui.button(label);
    let font_id = TextStyle::Body.resolve(ui.style());
    let color = ui.style().visuals.noninteractive().fg_stroke.color;

    let galley = ui.fonts().layout_job(LayoutJob::simple_singleline(
        shortcut.into(),
        font_id,
        color,
    ));
    ui.painter().galley(
        pos2(
            btn_re.rect.right() - galley.size().x - 2.0,
            btn_re.rect.top() + 2.0,
        ),
        galley,
    );
    btn_re
}
