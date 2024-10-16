use {
    super::{dialogs::JumpDialog, egui_ui_ext::EguiResponseExt as _, Gui},
    crate::{
        app::{interact_mode::InteractMode, App},
        meta::find_most_specific_region_for_offset,
        shell::msg_if_fail,
        util::human_size,
        view::ViewportVec,
    },
    egui::{text::LayoutJob, Align, Color32, DragValue, Stroke, TextFormat, TextStyle, Ui},
    slotmap::Key,
};

pub fn ui(ui: &mut Ui, app: &mut App, mouse_pos: ViewportVec, gui: &mut Gui) {
    ui.horizontal(|ui| {
        let job = key_label(ui, "F1", "View");
        if ui
            .selectable_label(app.hex_ui.interact_mode == InteractMode::View, job)
            .clicked()
        {
            app.hex_ui.interact_mode = InteractMode::View;
        }
        ui.style_mut().visuals.selection.bg_fill = Color32::from_rgb(168, 150, 32);
        let job = key_label(ui, "F2", "Edit");
        if ui
            .selectable_label(app.hex_ui.interact_mode == InteractMode::Edit, job)
            .clicked()
        {
            app.hex_ui.interact_mode = InteractMode::Edit;
        }
        ui.separator();
        let data_len = app.data.len();
        if data_len != 0 {
            if let Some(view_key) = app.hex_ui.focused_view {
                let view = &app.meta_state.meta.views[view_key].view;
                let per = match app.meta_state.meta.low.perspectives.get_mut(view.perspective) {
                    Some(per) => per,
                    None => {
                        ui.label("Invalid perspective key");
                        return;
                    }
                };
                ui.label("offset");
                ui.add(DragValue::new(
                    &mut app.meta_state.meta.low.regions[per.region].region.begin,
                ));
                ui.label("columns");
                ui.add(DragValue::new(&mut per.cols));
                let offsets = view.offsets(
                    &app.meta_state.meta.low.perspectives,
                    &app.meta_state.meta.low.regions,
                );
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "Precision is good until 52 bits (more than reasonable)"
                )]
                ui.label(format!(
                    "view offset: row {} col {} byte {} ({:.2}%)",
                    offsets.row,
                    offsets.col,
                    offsets.byte,
                    (offsets.byte as f64 / data_len as f64) * 100.0
                ))
                .on_hover_text_deferred(|| human_size(offsets.byte));
            }
        }
        ui.separator();
        let mut text = egui::RichText::new(format!(
            "cursor: {} ({:x})",
            app.edit_state.cursor, app.edit_state.cursor,
        ));
        let out_of_bounds = app.edit_state.cursor >= app.data.len();
        let cursor_end = app.edit_state.cursor == app.data.len().saturating_sub(1);
        let cursor_begin = app.edit_state.cursor == 0;
        if out_of_bounds {
            text = text.color(egui::Color32::RED);
        } else if cursor_end {
            text = text.color(egui::Color32::YELLOW);
        } else if cursor_begin {
            text = text.color(egui::Color32::GREEN);
        }
        let re = ui.label(text);
        re.context_menu(|ui| {
            if ui.button("Copy").clicked() {
                let result = app.clipboard.set_text(app.edit_state.cursor.to_string());
                msg_if_fail(result, "Failed to set clipboard text", &mut gui.msg_dialog);
                ui.close_menu();
            }
            if ui.button("Copy absolute").on_hover_text("Hard seek + cursor").clicked() {
                let result = app.clipboard.set_text(
                    (app.edit_state.cursor + app.src_args.hard_seek.unwrap_or(0)).to_string(),
                );
                msg_if_fail(result, "Failed to set clipboard text", &mut gui.msg_dialog);
                ui.close_menu();
            }
        });
        if re.clicked() {
            Gui::add_dialog(&mut gui.dialogs, JumpDialog::default());
        }
        if out_of_bounds {
            re.on_hover_text("Cursor is out of bounds");
        } else if cursor_end {
            re.on_hover_text("Cursor is at end of document");
        } else if cursor_begin {
            re.on_hover_text("Cursor is at beginning");
        } else {
            re.on_hover_text_deferred(|| human_size(app.edit_state.cursor));
        }
        if let Some(label) = app
            .meta_state
            .meta
            .bookmarks
            .iter()
            .find_map(|bm| (bm.offset == app.edit_state.cursor).then_some(bm.label.as_str()))
        {
            ui.label(egui::RichText::new(label).color(Color32::from_rgb(150, 170, 40)));
        }
        if let Some(region) = find_most_specific_region_for_offset(
            &app.meta_state.meta.low.regions,
            app.edit_state.cursor,
        ) {
            let reg = &app.meta_state.meta.low.regions[region];
            region_label(ui, &reg.name).context_menu(|ui| {
                if ui.button("Select").clicked() {
                    app.hex_ui.select_a = Some(reg.region.begin);
                    app.hex_ui.select_b = Some(reg.region.end);
                    ui.close_menu();
                }
            });
        }
        if !app.hex_ui.current_layout.is_null()
            && let Some((offset, _view_idx)) = app.byte_offset_at_pos(mouse_pos.x, mouse_pos.y)
        {
            ui.label(format!("mouse: {offset} ({offset:x})"));
            if let Some(region) =
                find_most_specific_region_for_offset(&app.meta_state.meta.low.regions, offset)
            {
                region_label(ui, &app.meta_state.meta.low.regions[region].name);
            }
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let mut txt = egui::RichText::new(format!("File size: {}", app.data.len()));
            let truncated = app.data.len() != app.orig_data_len;
            if truncated {
                txt = txt.color(egui::Color32::RED);
            }
            let label = egui::Label::new(txt).sense(egui::Sense::click());
            let mut label_re = ui.add(label).on_hover_ui(|ui| {
                ui.label("Click to copy");
                ui.label(format!(
                    "Human size: {}",
                    crate::util::human_size(app.data.len())
                ));
            });
            if truncated {
                label_re = label_re.on_hover_text_deferred(|| {
                    format!("Length changed, orig.: {}", app.orig_data_len)
                });
            }
            if label_re.clicked() {
                crate::app::set_clipboard_string(
                    &mut app.clipboard,
                    &mut gui.msg_dialog,
                    &app.data.len().to_string(),
                );
            }
        });
    });
}

fn region_label(ui: &mut Ui, name: &str) -> egui::Response {
    let label =
        egui::Label::new(egui::RichText::new(format!("[{name}]")).color(egui::Color32::LIGHT_BLUE))
            .sense(egui::Sense::click());
    ui.add(label)
}

/// A key "box" and then some text. Like `[F1] View`
fn key_label(ui: &Ui, key_text: &str, label_text: &str) -> LayoutJob {
    let mut job = LayoutJob::default();
    let style = ui.style();
    let body_font = TextStyle::Body.resolve(style);
    job.append(
        key_text,
        0.0,
        TextFormat {
            font_id: body_font.clone(),
            color: style.visuals.widgets.active.fg_stroke.color,
            background: style.visuals.code_bg_color,
            italics: false,
            underline: Stroke::NONE,
            strikethrough: Stroke::NONE,
            valign: Align::Center,
            ..Default::default()
        },
    );
    job.append(
        label_text,
        10.0,
        TextFormat::simple(body_font, style.visuals.widgets.active.fg_stroke.color),
    );
    job
}
