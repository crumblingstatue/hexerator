use std::hash::Hash;

use egui_sfml::egui::{self, emath::Numeric};

use crate::view::{ScrollOffset, View, ViewKind, ViewportRect};

#[derive(Debug)]
pub struct ViewsWindow {
    pub open: bool,
    new_kind: ViewKind,
}

impl Default for ViewsWindow {
    fn default() -> Self {
        Self {
            open: Default::default(),
            new_kind: ViewKind::Hex,
        }
    }
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
        let mut removed_idx = None;
        app.views.retain_mut(|view| {
            let mut retain = true;
            ui.group(|ui| {
                view_combo(egui::Id::new("view_combo").with(idx), &mut view.kind, ui);
                viewport_rect_ui(ui, &mut view.viewport_rect);
                labelled_drag(ui, "column width", &mut view.col_w);
                labelled_drag(ui, "row height", &mut view.row_h);
                ui.checkbox(&mut view.active, "Active");
                if ui.button("Delete").clicked() {
                    retain = false;
                    removed_idx = Some(idx);
                }
                idx += 1;
            });
            retain
        });
        if let Some(focused) = &mut app.focused_view && let Some(rem_idx) = removed_idx && *focused >= rem_idx {
            if app.views.is_empty() {
                app.focused_view = None;
            } else if *focused > 0 {
                *focused -= 1;
            }
        }
        ui.separator();
        view_combo("new_kind_combo", &mut app.ui.views_window.new_kind, ui);
        if ui.button("Add new").clicked() {
            app.views.push(View {
                viewport_rect: ViewportRect {
                    x: 0,
                    y: 0,
                    w: 100,
                    h: 100,
                },
                kind: std::mem::replace(&mut app.ui.views_window.new_kind, ViewKind::Hex),
                col_w: 32,
                row_h: 32,
                scroll_offset: ScrollOffset::default(),
                scroll_speed: 10,
                active: true,
            })
        }
    }
}

fn view_combo(id: impl Hash, kind: &mut crate::view::ViewKind, ui: &mut egui::Ui) {
    egui::ComboBox::new(id, "kind")
        .selected_text(kind.name())
        .show_ui(ui, |ui| {
            ui.selectable_value(kind, ViewKind::Ascii, ViewKind::Ascii.name());
            ui.selectable_value(kind, ViewKind::Hex, ViewKind::Hex.name());
            ui.selectable_value(kind, ViewKind::Block, ViewKind::Block.name());
        });
}

fn viewport_rect_ui(ui: &mut egui::Ui, viewport_rect: &mut ViewportRect) {
    labelled_drag(ui, "x", &mut viewport_rect.x);
    labelled_drag(ui, "y", &mut viewport_rect.y);
    labelled_drag(ui, "w", &mut viewport_rect.w);
    labelled_drag(ui, "h", &mut viewport_rect.h);
}

fn labelled_drag<T: Numeric>(ui: &mut egui::Ui, label: &str, val: &mut T) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.add(egui::DragValue::new(val));
    });
}
