use egui_sfml::egui;
use slotmap::Key;

use crate::{
    layout::{default_margin, Layout},
    meta::{LayoutKey, ViewKey},
};

use super::window_open::WindowOpen;

#[derive(Default)]
pub struct LayoutsWindow {
    pub open: WindowOpen,
    selected: LayoutKey,
    swap_a: ViewKey,
    edit_name: bool,
}
impl LayoutsWindow {
    pub(crate) fn ui(ui: &mut egui_sfml::egui::Ui, app: &mut crate::app::App) {
        let win = &mut app.ui.layouts_window;
        if win.open.just_now() {
            win.selected = app.current_layout;
        }
        for (k, v) in &app.meta.layouts {
            if ui.selectable_label(win.selected == k, &v.name).clicked() {
                win.selected = k;
                app.current_layout = k;
            }
        }
        if !win.selected.is_null() {
            ui.separator();
            let layout = &mut app.meta.layouts[win.selected];
            ui.horizontal(|ui| {
                if win.edit_name {
                    if ui.text_edit_singleline(&mut layout.name).lost_focus() {
                        win.edit_name = false;
                    }
                } else {
                    ui.heading(&layout.name);
                }
                if ui.button("✏").clicked() {
                    win.edit_name ^= true;
                }
            });
            let unused_views: Vec<ViewKey> = app
                .meta
                .views
                .keys()
                .filter(|&k| !layout.iter().any(|k2| k2 == k))
                .collect();
            egui::Grid::new("view_grid").show(ui, |ui| {
                let mut swap = None;
                layout.view_grid.retain_mut(|row| {
                    let mut retain_row = true;
                    row.retain_mut(|view_key| {
                        let mut retain = true;
                        let view = &app.meta.views[*view_key];
                        if win.swap_a == *view_key {
                            if ui.selectable_label(true, &view.name).clicked() {
                                win.swap_a = ViewKey::null();
                            }
                        } else if !win.swap_a.is_null() {
                            if ui.button(&format!("🔃 {}", view.name)).clicked() {
                                swap = Some((win.swap_a, *view_key));
                            }
                        } else {
                            ui.menu_button(&view.name, |ui| {
                                for &k in &unused_views {
                                    if ui.button(&app.meta.views[k].name).clicked() {
                                        *view_key = k;
                                        ui.close_menu();
                                    }
                                }
                            })
                            .response
                            .context_menu(|ui| {
                                if ui.button("🔃 Swap").clicked() {
                                    win.swap_a = *view_key;
                                    ui.close_menu();
                                }
                                if ui.button("🗑 Remove").clicked() {
                                    retain = false;
                                    ui.close_menu();
                                }
                                if ui.button("👁 View properties").clicked() {
                                    app.ui.views_window.open.set(true);
                                    app.ui.views_window.selected = *view_key;
                                    ui.close_menu();
                                }
                            });
                        }

                        retain
                    });
                    ui.add_enabled_ui(!unused_views.is_empty(), |ui| {
                        ui.menu_button("✚", |ui| {
                            for &k in &unused_views {
                                if ui.button(&app.meta.views[k].name).clicked() {
                                    row.push(k);
                                    ui.close_menu();
                                }
                            }
                        })
                        .response
                        .on_hover_text("Add view")
                        .on_disabled_hover_text("No views to add (all added)");
                    });
                    if ui.button("🗑").on_hover_text("Delete row").clicked() {
                        retain_row = false;
                    }
                    ui.end_row();
                    if row.is_empty() {
                        retain_row = false;
                    }
                    retain_row
                });
                if let Some((a, b)) = swap {
                    if let Some((a_row, a_col)) = layout.idx_of_key(a) {
                        if let Some((b_row, b_col)) = layout.idx_of_key(b) {
                            let addr_a = std::ptr::addr_of_mut!(layout.view_grid[a_row][a_col]);
                            let addr_b = std::ptr::addr_of_mut!(layout.view_grid[b_row][b_col]);
                            unsafe {
                                std::ptr::swap(addr_a, addr_b);
                            }
                            win.swap_a = ViewKey::null();
                        }
                    }
                }
                ui.add_enabled_ui(!unused_views.is_empty(), |ui| {
                    ui.menu_button("✚", |ui| {
                        for &k in &unused_views {
                            if ui.button(&app.meta.views[k].name).clicked() {
                                layout.view_grid.push(vec![k]);
                                ui.close_menu();
                            }
                        }
                    })
                    .response
                    .on_hover_text("Add view")
                    .on_disabled_hover_text("No views to add (all added)");
                });
            });
            ui.horizontal(|ui| {
                ui.label("Margin");
                ui.add(egui::DragValue::new(&mut layout.margin).clamp_range(3..=64));
            });
        }
        ui.separator();
        if ui.button("New layout").clicked() {
            let key = app.meta.layouts.insert(Layout {
                name: "New layout".into(),
                view_grid: Vec::new(),
                margin: default_margin(),
            });
            win.selected = key;
            app.current_layout = key;
        }
        win.open.post_ui();
    }
}
