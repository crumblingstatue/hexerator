use egui_sfml::egui::{self, emath::Numeric};

use crate::view::ViewportRect;

#[derive(Debug, Default)]
pub struct ViewsWindow {
    pub open: bool,
}
impl ViewsWindow {
    pub(crate) fn ui(ui: &mut egui_sfml::egui::Ui, app: &mut crate::app::App) {
        app.views.retain_mut(|view| {
            let mut retain = true;
            ui.group(|ui| {
                viewport_rect_ui(ui, &mut view.viewport_rect);
                if ui.button("Delete").clicked() {
                    retain = false;
                }
            });
            retain
        });
    }
}

fn viewport_rect_ui(ui: &mut egui::Ui, viewport_rect: &mut ViewportRect) {
    labelled_drag(ui, "x", &mut viewport_rect.x);
    labelled_drag(ui, "w", &mut viewport_rect.w);
}

fn labelled_drag<T: Numeric>(ui: &mut egui::Ui, label: &str, val: &mut T) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.add(egui::DragValue::new(val));
    });
}
