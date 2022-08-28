use egui_sfml::egui;
use slotmap::Key;

use crate::{
    app::LayoutKey,
    layout::{default_margin, Layout},
};

use super::window_open::WindowOpen;

#[derive(Default)]
pub struct LayoutsWindow {
    pub open: WindowOpen,
    selected: LayoutKey,
}
impl LayoutsWindow {
    pub(crate) fn ui(ui: &mut egui_sfml::egui::Ui, app: &mut crate::app::App) {
        for (k, v) in &app.view_layout_map {
            if ui
                .selectable_label(app.ui.layouts_window.selected == k, &v.name)
                .clicked()
            {
                app.ui.layouts_window.selected = k;
            }
        }
        if !app.ui.layouts_window.selected.is_null() {
            ui.separator();
            let layout = &mut app.view_layout_map[app.ui.layouts_window.selected];
            ui.heading(&layout.name);
            egui::Grid::new("view_grid").show(ui, |ui| {
                layout.view_grid.retain_mut(|row| {
                    let mut retain_row = true;
                    row.retain_mut(|view_key| {
                        let mut retain = true;
                        let view = &app.view_map[*view_key];
                        ui.menu_button(&view.name, |ui| {
                            for (k, v) in &app.view_map {
                                if ui.button(&v.name).clicked() {
                                    *view_key = k;
                                    ui.close_menu();
                                }
                            }
                            if ui.button("Delete").clicked() {
                                retain = false;
                                ui.close_menu();
                            }
                        });
                        retain
                    });
                    ui.menu_button("New view", |ui| {
                        for (k, v) in &app.view_map {
                            if ui.button(&v.name).clicked() {
                                row.push(k);
                                ui.close_menu();
                            }
                        }
                    });
                    if ui.button("Delete row").clicked() {
                        retain_row = false;
                    }
                    ui.end_row();
                    retain_row
                });
                if ui.button("New row").clicked() {
                    layout.view_grid.push(Vec::new());
                }
            });
        }
        ui.separator();
        if ui.button("New layout").clicked() {
            app.ui.layouts_window.selected = app.view_layout_map.insert(Layout {
                name: "New layout".into(),
                view_grid: Vec::new(),
                margin: default_margin(),
            });
        }
    }
}
