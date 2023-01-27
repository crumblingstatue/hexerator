use {
    super::window_open::WindowOpen,
    crate::{
        app::App,
        meta::{NamedView, ViewKey},
        region_context_menu,
        view::{HexData, TextData, TextKind, View, ViewKind},
    },
    egui_extras::{Column, TableBuilder},
    egui_sfml::{
        egui::{self, emath::Numeric},
        sfml::graphics::Font,
    },
    slotmap::Key,
    std::{hash::Hash, ops::RangeInclusive},
};

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
    pub(crate) fn ui(
        ui: &mut egui::Ui,
        gui: &mut crate::gui::Gui,
        app: &mut crate::app::App,
        font: &Font,
    ) {
        if gui.views_window.open.just_now() &&
           // Don't override selected key if there already is one
           // For example, it could be set by the context menu "view properties".
           gui.views_window.selected.is_null() &&
           let Some(view_key) = app.hex_ui.focused_view {
           gui.views_window.selected = view_key;
        }
        let mut removed_idx = None;
        if app.meta_state.meta.views.is_empty() {
            ui.label("No views");
            return;
        }
        TableBuilder::new(ui)
            .columns(Column::auto(), 3)
            .column(Column::remainder())
            .resizable(true)
            .header(24.0, |mut row| {
                row.col(|ui| {
                    ui.label("Name");
                });
                row.col(|ui| {
                    ui.label("Kind");
                });
                row.col(|ui| {
                    ui.label("Perspective");
                });
                row.col(|ui| {
                    ui.label("Region");
                });
            })
            .body(|body| {
                let keys: Vec<ViewKey> = app.meta_state.meta.views.keys().collect();
                let mut action = Action::None;
                body.rows(20.0, keys.len(), |idx, mut row| {
                    let view_key = keys[idx];
                    let view = &app.meta_state.meta.views[view_key];
                    row.col(|ui| {
                        let ctx_menu = |ui: &mut egui::Ui| {
                            ui.menu_button("Containing layouts", |ui| {
                                for (key, layout) in app.meta_state.meta.layouts.iter() {
                                    if layout.contains_view(view_key)
                                        && ui.button(&layout.name).clicked()
                                    {
                                        App::switch_layout(
                                            &mut app.hex_ui,
                                            &app.meta_state.meta,
                                            key,
                                        );
                                        app.hex_ui.focused_view = Some(view_key);
                                        ui.close_menu();
                                    }
                                }
                            });
                        };
                        if ui
                            .selectable_label(view_key == gui.views_window.selected, &view.name)
                            .context_menu(ctx_menu)
                            .clicked()
                        {
                            gui.views_window.selected = view_key;
                        }
                    });
                    row.col(|ui| {
                        ui.label(egui::RichText::new(view.view.kind.name()).code());
                    });
                    row.col(|ui| {
                        if ui
                            .link(&app.meta_state.meta.low.perspectives[view.view.perspective].name)
                            .clicked()
                        {
                            gui.perspectives_window.open.set(true);
                        }
                    });
                    row.col(|ui| {
                        let per = &app.meta_state.meta.low.perspectives[view.view.perspective];
                        let reg = &app.meta_state.meta.low.regions[per.region];
                        let ctx_menu =
                            |ui: &mut egui::Ui| region_context_menu!(ui, app, reg, action);
                        if ui
                            .link(&reg.name)
                            .on_hover_text(&reg.desc)
                            .context_menu(ctx_menu)
                            .clicked()
                        {
                            gui.regions_window.open.set(true);
                            gui.regions_window.selected_key = Some(per.region);
                        }
                    });
                });
                match action {
                    Action::None => {}
                    Action::Goto(off) => {
                        app.edit_state.cursor = off;
                        app.center_view_on_offset(off);
                        app.hex_ui.flash_cursor();
                    }
                }
            });
        ui.separator();
        ui.menu_button("New from perspective", |ui| {
            for (key, perspective) in app.meta_state.meta.low.perspectives.iter() {
                if ui.button(&perspective.name).clicked() {
                    ui.close_menu();
                    app.meta_state.meta.views.insert(NamedView {
                        view: View::new(ViewKind::Hex(HexData::default()), key),
                        name: perspective.name.to_owned(),
                    });
                }
            }
        });
        ui.separator();
        if let Some(view) = app.meta_state.meta.views.get_mut(gui.views_window.selected) {
            ui.horizontal(|ui| {
                if gui.views_window.rename {
                    if ui
                        .add(egui::TextEdit::singleline(&mut view.name).desired_width(150.0))
                        .lost_focus()
                    {
                        gui.views_window.rename = false;
                    }
                } else {
                    ui.heading(&view.name);
                }
                if ui.button("✏").on_hover_text("Rename").clicked() {
                    gui.views_window.rename ^= true;
                }
                if view_combo(egui::Id::new("view_combo"), &mut view.view.kind, ui, font) {
                    view.view.adjust_state_to_kind();
                }
            });
            egui::ComboBox::new("new_perspective_combo", "Perspective")
                .selected_text(&app.meta_state.meta.low.perspectives[view.view.perspective].name)
                .show_ui(ui, |ui| {
                    for k in app.meta_state.meta.low.perspectives.keys() {
                        if ui
                            .selectable_label(
                                k == view.view.perspective,
                                &app.meta_state.meta.low.perspectives[k].name,
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
                removed_idx = Some(gui.views_window.selected);
            }
        }
        if let Some(rem_key) = removed_idx {
            app.meta_state.meta.remove_view(rem_key);
            app.hex_ui.focused_view = None;
        }
        gui.views_window.open.post_ui();
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

enum Action {
    None,
    Goto(usize),
}
