use std::{hash::Hash, ops::RangeInclusive};

use egui_sfml::egui::{self, emath::Numeric};
use sfml::graphics::Font;

use crate::view::{HexData, TextData, TextKind, View, ViewKind, ViewportRect};

#[derive(Debug)]
pub struct ViewsWindow {
    pub open: bool,
    new_kind: ViewKind,
}

impl Default for ViewsWindow {
    fn default() -> Self {
        Self {
            open: Default::default(),
            new_kind: ViewKind::Hex(HexData::default()),
        }
    }
}

impl ViewKind {
    const HEX_NAME: &str = "Hex";
    const DEC_NAME: &str = "Decimal";
    const TEXT_NAME: &str = "Text";
    const BLOCK_NAME: &str = "Block";
    fn name(&self) -> &'static str {
        match *self {
            ViewKind::Hex(_) => Self::HEX_NAME,
            ViewKind::Dec(_) => Self::DEC_NAME,
            ViewKind::Text(_) => Self::TEXT_NAME,
            ViewKind::Block => Self::BLOCK_NAME,
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
                if view_combo(
                    egui::Id::new("view_combo").with(idx),
                    &mut view.kind,
                    ui,
                    font,
                ) {
                    view.adjust_state_to_kind();
                }
                let mut adjust_block_size = false;
                match &mut view.kind {
                    ViewKind::Hex(HexData { font_size, .. })
                    | ViewKind::Dec(HexData { font_size, .. })
                    | ViewKind::Text(TextData { font_size, .. }) => {
                        ui.horizontal(|ui| {
                            ui.label("Font size");
                            if ui
                                .add(
                                    egui::DragValue::new(font_size)
                                        .clamp_range(MIN_FONT_SIZE..=MAX_FONT_SIZE),
                                )
                                .changed()
                            {
                                adjust_block_size = true;
                            };
                        });
                        if let ViewKind::Text(text) = &mut view.kind {
                            let mut changed = false;
                            egui::ComboBox::new(egui::Id::new("text_combo").with(idx), "Text kind")
                                .selected_text(text.text_kind.name())
                                .show_ui(ui, |ui| {
                                    changed |= ui
                                        .selectable_value(
                                            &mut text.text_kind,
                                            TextKind::Ascii,
                                            TextKind::Ascii.name(),
                                        )
                                        .clicked();
                                    changed |= ui
                                        .selectable_value(
                                            &mut text.text_kind,
                                            TextKind::Utf16Le,
                                            TextKind::Utf16Le.name(),
                                        )
                                        .clicked();
                                    changed |= ui
                                        .selectable_value(
                                            &mut text.text_kind,
                                            TextKind::Utf16Be,
                                            TextKind::Utf16Be.name(),
                                        )
                                        .clicked();
                                });
                            if changed {
                                view.bytes_per_block = text.text_kind.bytes_needed();
                            }
                        }
                    }
                    ViewKind::Block => {}
                }
                if adjust_block_size {
                    view.adjust_block_size();
                    #[expect(
                        clippy::cast_possible_truncation,
                        clippy::cast_sign_loss,
                        reason = "It's extremely unlikely line spacing is not between 0 and i16::MAX"
                    )]
                    if let ViewKind::Text(data) = &mut view.kind {
                        data.line_spacing = font.line_spacing(u32::from(data.font_size)) as u16;
                    }
                }
                viewport_rect_ui(ui, &mut view.viewport_rect);
                labelled_drag(ui, "column width", &mut view.col_w, 1..=128);
                labelled_drag(ui, "row height", &mut view.row_h, 1..=128);
                labelled_drag(ui, "bytes per block", &mut view.bytes_per_block, 1..=64);
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
        view_combo(
            "new_kind_combo",
            &mut app.ui.views_window.new_kind,
            ui,
            font,
        );
        if ui.button("Add new").clicked() {
            app.views.push(View::new(
                std::mem::replace(
                    &mut app.ui.views_window.new_kind,
                    ViewKind::Hex(HexData::default()),
                ),
                0,
                0,
                100,
                100,
            ));
        }
    }
}

/// Returns whether the value was changed
fn view_combo(
    id: impl Hash,
    kind: &mut crate::view::ViewKind,
    ui: &mut egui::Ui,
    font: &Font,
) -> bool {
    let mut changed = false;
    egui::ComboBox::new(id, "kind")
        .selected_text(kind.name())
        .show_ui(ui, |ui| {
            if ui
                .selectable_label(kind.name() == ViewKind::HEX_NAME, ViewKind::HEX_NAME)
                .clicked()
            {
                *kind = ViewKind::Hex(HexData::default());
                changed = true;
            }
            if ui
                .selectable_label(kind.name() == ViewKind::DEC_NAME, ViewKind::DEC_NAME)
                .clicked()
            {
                *kind = ViewKind::Dec(HexData::default());
                changed = true;
            }
            if ui
                .selectable_label(kind.name() == ViewKind::TEXT_NAME, ViewKind::TEXT_NAME)
                .clicked()
            {
                *kind = ViewKind::Text(TextData::default_from_font(font, 14));
                changed = true;
            }
            if ui
                .selectable_label(kind.name() == ViewKind::BLOCK_NAME, ViewKind::BLOCK_NAME)
                .clicked()
            {
                *kind = ViewKind::Block;
                changed = true;
            }
        });
    changed
}

fn viewport_rect_ui(ui: &mut egui::Ui, viewport_rect: &mut ViewportRect) {
    labelled_drag(ui, "x", &mut viewport_rect.x, None);
    labelled_drag(ui, "y", &mut viewport_rect.y, None);
    labelled_drag(ui, "w", &mut viewport_rect.w, None);
    labelled_drag(ui, "h", &mut viewport_rect.h, None);
}

fn labelled_drag<T: Numeric>(
    ui: &mut egui::Ui,
    label: &str,
    val: &mut T,
    range: impl Into<Option<RangeInclusive<T>>>,
) -> egui::Response {
    ui.horizontal(|ui| {
        ui.label(label);
        let mut dv = egui::DragValue::new(val);
        if let Some(range) = range.into() {
            dv = dv.clamp_range(range);
        }
        ui.add(dv)
    })
    .inner
}
