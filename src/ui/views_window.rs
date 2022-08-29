use std::{hash::Hash, ops::RangeInclusive};

use egui_sfml::egui::{self, emath::Numeric};
use egui_sfml::sfml::graphics::Font;
use slotmap::Key;

use crate::meta::{NamedView, PerspectiveKey, PerspectiveMap, RegionMap, ViewKey};
use crate::view::{HexData, TextData, TextKind, View, ViewKind};

use super::window_open::WindowOpen;

#[derive(Default)]
pub struct ViewsWindow {
    pub open: WindowOpen,
    pub selected: ViewKey,
    rename: bool,
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
        if app.ui.views_window.open.just_opened() && let Some(view_key) = app.focused_view {
            app.ui.views_window.selected = view_key;
        }
        let mut removed_idx = None;
        ui.heading("Views");
        if app.meta.views.is_empty() {
            ui.label("No views");
            return;
        }
        for (k, view) in app.meta.views.iter_mut() {
            ui.horizontal(|ui| {
                if ui
                    .selectable_label(k == app.ui.views_window.selected, &view.name)
                    .clicked()
                {
                    app.ui.views_window.selected = k;
                }
                ui.label(egui::RichText::new(view.view.kind.name()).code());
            });
        }
        ui.separator();
        if ui.button("Add new").clicked() {
            let k = app.meta.views.insert(NamedView {
                view: View::new(ViewKind::Hex(HexData::default()), PerspectiveKey::null()),
                name: "Unnamed view".into(),
            });
            app.meta.layouts[app.current_layout].view_grid[0].push(k);
            app.resize_views.reset();
        }
        ui.separator();
        if let Some(view) = app.meta.views.get_mut(app.ui.views_window.selected) {
            ui.horizontal(|ui| {
                if app.ui.views_window.rename {
                    if ui
                        .add(egui::TextEdit::singleline(&mut view.name).desired_width(150.0))
                        .lost_focus()
                    {
                        app.ui.views_window.rename = false;
                    }
                } else {
                    ui.heading(&view.name);
                }
                if ui.button("✏").on_hover_text("Rename").clicked() {
                    app.ui.views_window.rename ^= true;
                }
                if view_combo(egui::Id::new("view_combo"), &mut view.view.kind, ui, font) {
                    view.view.adjust_state_to_kind();
                }
            });
            egui::ComboBox::new("new_perspective_combo", "Perspective")
                .selected_text(perspective_label(
                    &app.meta.perspectives,
                    &app.meta.regions,
                    view.view.perspective,
                ))
                .show_ui(ui, |ui| {
                    for k in app.meta.perspectives.keys() {
                        if ui
                            .selectable_label(
                                k == view.view.perspective,
                                perspective_label(&app.meta.perspectives, &app.meta.regions, k),
                            )
                            .clicked()
                        {
                            view.view.perspective = k;
                        }
                    }
                });
            ui.group(|ui| {
                let mut adjust_block_size = false;
                match &mut view.view.kind {
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
                        if let ViewKind::Text(text) = &mut view.view.kind {
                            let mut changed = false;
                            egui::ComboBox::new(egui::Id::new("text_combo"), "Text kind")
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
                                view.view.bytes_per_block = text.text_kind.bytes_needed();
                            }
                        }
                    }
                    ViewKind::Block => {}
                }
                if adjust_block_size {
                    view.view.adjust_block_size();
                    #[expect(
                        clippy::cast_possible_truncation,
                        clippy::cast_sign_loss,
                        reason = "It's extremely unlikely line spacing is not between 0 and i16::MAX"
                    )]
                    if let ViewKind::Text(data) = &mut view.view.kind {
                        data.line_spacing = font.line_spacing(u32::from(data.font_size)) as u16;
                    }
                }
                ui.horizontal(|ui| {
                    labelled_drag(ui, "col w", &mut view.view.col_w, 1..=128);
                    labelled_drag(ui, "row h", &mut view.view.row_h, 1..=128);
                });
                labelled_drag(
                    ui,
                    "bytes per block",
                    &mut view.view.bytes_per_block,
                    1..=64,
                );
            });
            if ui.button("Delete").clicked() {
                removed_idx = Some(app.ui.views_window.selected);
                app.resize_views.reset();
            }
        }
        if let Some(rem_key) = removed_idx {
            app.meta.views.remove(rem_key);
            app.focused_view = None;
        }
        app.ui.views_window.open.post_ui();
    }
}

/// Try to give a sensible label for a perspective
fn perspective_label(
    app_perspectives: &PerspectiveMap,
    app_regions: &RegionMap,
    perspective_key: PerspectiveKey,
) -> String {
    if perspective_key.is_null() {
        return "<null perspective key>".into();
    }
    let p = &app_perspectives[perspective_key];
    let r = &app_regions[p.region];
    format!("{}:{}", r.name, p.cols)
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
