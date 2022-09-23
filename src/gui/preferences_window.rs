use {
    super::{window_open::WindowOpen, Gui},
    crate::{app::App, config},
    egui,
};

#[derive(Default)]
pub struct PreferencesWindow {
    pub open: WindowOpen,
}

impl PreferencesWindow {
    pub fn ui(ui: &mut egui::Ui, _gui: &mut Gui, app: &mut App) {
        let style = &mut app.cfg.style;
        ui.heading("Font sizes");
        let mut any_changed = false;
        ui.horizontal(|ui| {
            ui.label("heading");
            any_changed |= ui
                .add(egui::DragValue::new(&mut style.font_sizes.heading))
                .changed();
        });
        ui.horizontal(|ui| {
            ui.label("body");
            any_changed |= ui
                .add(egui::DragValue::new(&mut style.font_sizes.body))
                .changed();
        });
        ui.horizontal(|ui| {
            ui.label("monospace");
            any_changed |= ui
                .add(egui::DragValue::new(&mut style.font_sizes.monospace))
                .changed();
        });
        ui.horizontal(|ui| {
            ui.label("button");
            any_changed |= ui
                .add(egui::DragValue::new(&mut style.font_sizes.button))
                .changed();
        });
        ui.horizontal(|ui| {
            ui.label("small");
            any_changed |= ui
                .add(egui::DragValue::new(&mut style.font_sizes.small))
                .changed();
        });
        if ui.button("Reset default").clicked() {
            *style = config::Style::default();
            any_changed = true;
        }
        if any_changed {
            crate::gui::set_font_sizes_ctx(ui.ctx(), style);
        }
    }
}
