pub trait EguiResponseExt {
    fn on_hover_text_deferred<F, R>(self, text_fun: F) -> Self
    where
        F: FnOnce() -> R,
        R: Into<egui::WidgetText>;
}

impl EguiResponseExt for egui::Response {
    fn on_hover_text_deferred<F, R>(self, text_fun: F) -> Self
    where
        F: FnOnce() -> R,
        R: Into<egui::WidgetText>,
    {
        // Yoinked from egui source
        self.on_hover_ui(|ui| {
            // Prevent `Area` auto-sizing from shrinking tooltips with dynamic content.
            // See https://github.com/emilk/egui/issues/5167
            ui.set_max_width(ui.spacing().tooltip_width);

            ui.add(egui::Label::new(text_fun()));
        })
    }
}
