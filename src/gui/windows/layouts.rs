use {
    super::{WinCtx, WindowOpen},
    crate::{
        app::App,
        meta::{LayoutKey, LayoutMapExt as _, MetaLow, NamedView, ViewKey, ViewMap},
        view::{HexData, TextData, View, ViewKind},
    },
    constcat::concat,
    egui_phosphor::regular as ic,
    egui_sf2g::sf2g::graphics::Font,
    slotmap::Key as _,
};

const L_NEW_FROM_PERSPECTIVE: &str = concat!(ic::PLUS, " New from perspective");
const L_HEX: &str = concat!(ic::HEXAGON, " Hex");
const L_TEXT: &str = concat!(ic::TEXT_AA, " Text");
const L_BLOCK: &str = concat!(ic::RECTANGLE, " Block");
const L_ADD_TO_NEW_ROW: &str = concat!(ic::PLUS, ic::ARROW_BEND_DOWN_RIGHT);
const L_ADD_TO_CURRENT_ROW: &str = concat!(ic::PLUS, ic::ARROW_LEFT);

#[derive(Default)]
pub struct LayoutsWindow {
    pub open: WindowOpen,
    selected: LayoutKey,
    swap_a: ViewKey,
    edit_name: bool,
}

impl super::Window for LayoutsWindow {
    fn ui(
        &mut self,
        WinCtx {
            ui,
            gui,
            app,
            font_size,
            font,
            ..
        }: WinCtx,
    ) {
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
                            ui.menu_button([ic::EYE, " ", view.name.as_str()].concat(), |ui| {
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
                    ui.menu_button(L_ADD_TO_CURRENT_ROW, |ui| {
                        for &k in &unused_views {
                            if ui
                                .button(
                                    [ic::EYE, " ", app.meta_state.meta.views[k].name.as_str()]
                                        .concat(),
                                )
                                .clicked()
                            {
                                row.push(k);
                                ui.close_menu();
                            }
                        }
                        if let Some(k) = add_new_view_menu(
                            ui,
                            &app.meta_state.meta.low,
                            &mut app.meta_state.meta.views,
                            font_size,
                            font,
                        ) {
                            row.push(k);
                            ui.close_menu();
                        }
                    })
                    .response
                    .on_hover_text("Add to current row");
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
                    if let Some([a_row, a_col]) = layout.idx_of_key(a) {
                        if let Some([b_row, b_col]) = layout.idx_of_key(b) {
                            let addr_a = &raw mut layout.view_grid[a_row][a_col];
                            let addr_b = &raw mut layout.view_grid[b_row][b_col];
                            // Safety: `addr_a` and `addr_b` are r/w valid and well-aligned
                            unsafe {
                                std::ptr::swap(addr_a, addr_b);
                            }
                            self.swap_a = ViewKey::null();
                        }
                    }
                }
                ui.menu_button(L_ADD_TO_NEW_ROW, |ui| {
                    for &k in &unused_views {
                        if ui
                            .button(
                                [ic::EYE, " ", app.meta_state.meta.views[k].name.as_str()].concat(),
                            )
                            .clicked()
                        {
                            layout.view_grid.push(vec![k]);
                            ui.close_menu();
                        }
                    }
                    if let Some(k) = add_new_view_menu(
                        ui,
                        &app.meta_state.meta.low,
                        &mut app.meta_state.meta.views,
                        font_size,
                        font,
                    ) {
                        layout.view_grid.push(vec![k]);
                        app.hex_ui.focused_view = Some(k);
                        ui.close_menu();
                    }
                })
                .response
                .on_hover_text("Add to new row")
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
            let key = app.meta_state.meta.layouts.add_new_default();
            self.selected = key;
            App::switch_layout(&mut app.hex_ui, &app.meta_state.meta, key);
        }
    }

    fn title(&self) -> &str {
        "Layouts"
    }
}

fn add_new_view_menu(
    ui: &mut egui::Ui,
    low: &MetaLow,
    views: &mut ViewMap,
    font_size: u16,
    font: &Font,
) -> Option<ViewKey> {
    let mut ret_key = None;
    ui.separator();
    ui.menu_button(L_NEW_FROM_PERSPECTIVE, |ui| {
        for (per_key, per) in &low.perspectives {
            ui.menu_button([ic::PERSPECTIVE, " ", per.name.as_str()].concat(), |ui| {
                let mut new = None;
                if ui.button(L_HEX).clicked() {
                    let view =
                        View::new(ViewKind::Hex(HexData::with_font_size(font_size)), per_key);
                    new = Some(("hex", view));
                }
                if ui.button(L_TEXT).clicked() {
                    let view = View::new(
                        #[expect(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
                        ViewKind::Text(TextData::with_font_info(
                            font.line_spacing(font_size.into()) as _,
                            font_size,
                        )),
                        per_key,
                    );
                    new = Some(("text", view));
                }
                if ui.button(L_BLOCK).clicked() {
                    let view = View::new(ViewKind::Block, per_key);
                    new = Some(("block", view));
                }
                if let Some((label, view)) = new {
                    let view_key = views.insert(NamedView {
                        view,
                        name: [per.name.as_str(), " ", label].concat(),
                    });
                    ret_key = Some(view_key);
                }
            });
        }
    });
    ret_key
}
