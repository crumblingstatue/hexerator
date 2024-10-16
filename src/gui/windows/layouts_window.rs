use {
    super::{WinCtx, WindowOpen},
    crate::{
        app::App,
        layout::{default_margin, Layout},
        meta::{LayoutKey, MetaLow, NamedView, ViewKey, ViewMap},
        view::{HexData, View, ViewKind},
    },
    slotmap::Key,
};

#[derive(Default)]
pub struct LayoutsWindow {
    pub open: WindowOpen,
    selected: LayoutKey,
    swap_a: ViewKey,
    edit_name: bool,
}
impl super::Window for LayoutsWindow {
    fn ui(&mut self, WinCtx { ui, gui, app, .. }: WinCtx) {
        if self.open.just_now() {
            self.selected = app.hex_ui.current_layout;
        }
        for (k, v) in &app.meta_state.meta.layouts {
            if ui.selectable_label(self.selected == k, &v.name).clicked() {
                self.selected = k;
                App::switch_layout(&mut app.hex_ui, &app.meta_state.meta, k);
            }
        }
        if !self.selected.is_null() {
            ui.separator();
            let Some(layout) = app.meta_state.meta.layouts.get_mut(self.selected) else {
                self.selected = LayoutKey::null();
                return;
            };
            ui.horizontal(|ui| {
                if self.edit_name {
                    if ui.text_edit_singleline(&mut layout.name).lost_focus() {
                        self.edit_name = false;
                    }
                } else {
                    ui.heading(&layout.name);
                }
                if ui.button("✏").clicked() {
                    self.edit_name ^= true;
                }
            });
            let unused_views: Vec<ViewKey> = app
                .meta_state
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
                        let view = &app.meta_state.meta.views[*view_key];
                        if self.swap_a == *view_key {
                            if ui.selectable_label(true, &view.name).clicked() {
                                self.swap_a = ViewKey::null();
                            }
                        } else if !self.swap_a.is_null() {
                            if ui.button(format!("🔃 {}", view.name)).clicked() {
                                swap = Some((self.swap_a, *view_key));
                            }
                        } else {
                            ui.menu_button(&view.name, |ui| {
                                for &k in &unused_views {
                                    if ui.button(&app.meta_state.meta.views[k].name).clicked() {
                                        *view_key = k;
                                        ui.close_menu();
                                    }
                                }
                                if unused_views.is_empty() {
                                    ui.label(egui::RichText::new("No unused views").italics());
                                }
                            })
                            .response
                            .context_menu(|ui| {
                                if ui.button("🔃 Swap").clicked() {
                                    self.swap_a = *view_key;
                                    ui.close_menu();
                                }
                                if ui.button("🗑 Remove").clicked() {
                                    retain = false;
                                    ui.close_menu();
                                }
                                if ui.button("👁 View properties").clicked() {
                                    gui.win.views.open.set(true);
                                    gui.win.views.selected = *view_key;
                                    ui.close_menu();
                                }
                            });
                        }

                        retain
                    });
                    ui.add_enabled_ui(!unused_views.is_empty(), |ui| {
                        ui.menu_button("✚", |ui| {
                            for &k in &unused_views {
                                if ui.button(&app.meta_state.meta.views[k].name).clicked() {
                                    row.push(k);
                                    ui.close_menu();
                                }
                            }
                            if let Some(k) = add_new_view_menu(
                                ui,
                                &app.meta_state.meta.low,
                                &mut app.meta_state.meta.views,
                            ) {
                                row.push(k);
                                ui.close_menu();
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
                            self.swap_a = ViewKey::null();
                        }
                    }
                }
                ui.add_enabled_ui(!unused_views.is_empty(), |ui| {
                    ui.menu_button("✚", |ui| {
                        for &k in &unused_views {
                            if ui.button(&app.meta_state.meta.views[k].name).clicked() {
                                layout.view_grid.push(vec![k]);
                                ui.close_menu();
                            }
                        }
                        if let Some(k) = add_new_view_menu(
                            ui,
                            &app.meta_state.meta.low,
                            &mut app.meta_state.meta.views,
                        ) {
                            layout.view_grid.push(vec![k]);
                            ui.close_menu();
                        }
                    })
                    .response
                    .on_hover_text("Add view")
                    .on_disabled_hover_text("No views to add (all added)");
                });
            });
            ui.horizontal(|ui| {
                ui.label("Margin");
                ui.label("x");
                ui.add(egui::DragValue::new(&mut layout.margin.x).range(3..=64));
                ui.label("y");
                ui.add(egui::DragValue::new(&mut layout.margin.y).range(3..=64));
            });
        }
        ui.separator();
        if ui.button("New layout").clicked() {
            let key = app.meta_state.meta.layouts.insert(Layout {
                name: "New layout".into(),
                view_grid: Vec::new(),
                margin: default_margin(),
            });
            self.selected = key;
            App::switch_layout(&mut app.hex_ui, &app.meta_state.meta, key);
        }
    }

    fn title(&self) -> &str {
        "Layouts"
    }
}

fn add_new_view_menu(ui: &mut egui::Ui, low: &MetaLow, views: &mut ViewMap) -> Option<ViewKey> {
    let mut ret_key = None;
    ui.separator();
    ui.menu_button("New from perspective", |ui| {
        for (k, per) in &low.perspectives {
            if ui.button(&per.name).clicked() {
                let key = views.insert(NamedView {
                    view: View::new(ViewKind::Hex(HexData::default()), k),
                    name: per.name.to_owned(),
                });
                ret_key = Some(key);
            }
        }
    });
    ret_key
}
