use std::hash::Hash;

use egui_sfml::egui::{self, emath::Numeric};
use sfml::graphics::Font;

use crate::view::{TextKind, View, ViewKind, ViewportRect};

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
    const fn name(&self) -> &'static str {
        match *self {
            Self::Hex => "Hex",
            Self::Dec => "Decimal",
            Self::Text => "Text",
            Self::Block => "Block",
        }
    }
}

pub const MIN_FONT_SIZE: u16 = 5;
pub const MAX_FONT_SIZE: u16 = 256;

impl ViewsWindow {
    pub(crate) fn ui(ui: &mut egui_sfml::egui::Ui, app: &mut crate::app::App, font: &Font) {
        let mut idx = 0;
        let mut removed_idx = None;
        app.views.retain_mut(|view| {
            let mut retain = true;
            ui.group(|ui| {
                if view_combo(egui::Id::new("view_combo").with(idx), &mut view.kind, ui) {
                    view.adjust_state_to_kind();
                }
                match view.kind {
                    ViewKind::Hex => {}
                    ViewKind::Dec => {}
                    ViewKind::Text => {
                        let mut changed = false;
                        egui::ComboBox::new(egui::Id::new("text_combo").with(idx), "Text kind")
                            .selected_text(view.text_kind.name())
                            .show_ui(ui, |ui| {
                                changed |= ui
                                    .selectable_value(
                                        &mut view.text_kind,
                                        TextKind::Ascii,
                                        TextKind::Ascii.name(),
                                    )
                                    .clicked();
                                changed |= ui
                                    .selectable_value(
                                        &mut view.text_kind,
                                        TextKind::Utf16Le,
                                        TextKind::Utf16Le.name(),
                                    )
                                    .clicked();
                                changed |= ui
                                    .selectable_value(
                                        &mut view.text_kind,
                                        TextKind::Utf16Be,
                                        TextKind::Utf16Be.name(),
                                    )
                                    .clicked();
                            });
                        if changed {
                            view.bytes_per_block = view.text_kind.bytes_needed();
                        }
                    }
                    ViewKind::Block => {}
                }
                viewport_rect_ui(ui, &mut view.viewport_rect);
                labelled_drag(ui, "column width", &mut view.col_w);
                labelled_drag(ui, "row height", &mut view.row_h);
                ui.horizontal(|ui| {
                    ui.label("Font size");
                    #[expect(
                        clippy::cast_possible_truncation,
                        clippy::cast_sign_loss,
                        reason = "It's extremely unlikely line spacing is not between 0 and i16::MAX"
                    )]
                    if ui
                        .add(
                            egui::DragValue::new(&mut view.font_size)
                                .clamp_range(MIN_FONT_SIZE..=MAX_FONT_SIZE),
                        )
                        .changed()
                    {
                        let line_spacing = font.line_spacing(u32::from(view.font_size));
                        view.line_spacing = line_spacing as u16;
                        view.adjust_block_size();
                    }
                });

                labelled_drag(ui, "bytes per block", &mut view.bytes_per_block);
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
            app.views.push(View::new(
                std::mem::replace(&mut app.ui.views_window.new_kind, ViewKind::Hex),
                0,
                0,
                100,
                100,
                font,
            ));
        }
    }
}

/// Returns whether the value was changed
fn view_combo(id: impl Hash, kind: &mut crate::view::ViewKind, ui: &mut egui::Ui) -> bool {
    let mut changed = false;
    egui::ComboBox::new(id, "kind")
        .selected_text(kind.name())
        .show_ui(ui, |ui| {
            changed |= ui
                .selectable_value(kind, ViewKind::Hex, ViewKind::Hex.name())
                .clicked();
            changed |= ui
                .selectable_value(kind, ViewKind::Dec, ViewKind::Dec.name())
                .clicked();
            changed |= ui
                .selectable_value(kind, ViewKind::Text, ViewKind::Text.name())
                .clicked();
            changed |= ui
                .selectable_value(kind, ViewKind::Block, ViewKind::Block.name())
                .clicked();
        });
    changed
}

fn viewport_rect_ui(ui: &mut egui::Ui, viewport_rect: &mut ViewportRect) {
    labelled_drag(ui, "x", &mut viewport_rect.x);
    labelled_drag(ui, "y", &mut viewport_rect.y);
    labelled_drag(ui, "w", &mut viewport_rect.w);
    labelled_drag(ui, "h", &mut viewport_rect.h);
}

fn labelled_drag<T: Numeric>(ui: &mut egui::Ui, label: &str, val: &mut T) -> egui::Response {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.add(egui::DragValue::new(val))
    })
    .inner
}
