use egui_sfml::egui::{self, emath::Numeric};

use crate::view::{ViewKind, ViewportRect};

#[derive(Debug, Default)]
pub struct ViewsWindow {
    pub open: bool,
}

impl ViewKind {
    fn name(&self) -> &'static str {
        match *self {
            ViewKind::Hex => "Hex",
            ViewKind::Ascii => "Ascii",
            ViewKind::Block => "Block",
        }
    }
}

impl ViewsWindow {
    pub(crate) fn ui(ui: &mut egui_sfml::egui::Ui, app: &mut crate::app::App) {
        let mut idx = 0;
        app.views.retain_mut(|view| {
            let mut retain = true;
            ui.group(|ui| {
                egui::ComboBox::new(egui::Id::new("view_combo").with(idx), "kind")
                    .selected_text(view.kind.name())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut view.kind,
                            ViewKind::Ascii,
                            ViewKind::Ascii.name(),
                        );
                        ui.selectable_value(&mut view.kind, ViewKind::Hex, ViewKind::Hex.name());
                        ui.selectable_value(
                            &mut view.kind,
                            ViewKind::Block,
                            ViewKind::Block.name(),
                        );
                    });
                viewport_rect_ui(ui, &mut view.viewport_rect);
                if ui.button("Delete").clicked() {
                    retain = false;
                }
                idx += 1;
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
