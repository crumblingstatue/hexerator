use {
    super::message_dialog::MessageDialog,
    crate::{
        app::{interact_mode::InteractMode, App},
        meta::find_most_specific_region_for_offset,
        view::ViewportVec,
    },
    egui::{text::LayoutJob, Align, Color32, DragValue, Stroke, TextFormat, TextStyle, Ui},
    slotmap::Key,
};

pub fn ui(ui: &mut Ui, app: &mut App, mouse_pos: ViewportVec, msg: &mut MessageDialog) {
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
                let per = match app
                    .meta_state
                    .meta
                    .low
                    .perspectives
                    .get_mut(view.perspective)
                {
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
                ));
            }
        }
        ui.separator();
        ui.label(format!(
            "cursor: {} ({:x})",
            app.edit_state.cursor, app.edit_state.cursor,
        ));
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
            let mut label_re = ui.add(label).on_hover_text("Click to copy");
            if truncated {
                label_re =
                    label_re.on_hover_text(format!("Length changed, orig.: {}", app.orig_data_len));
            }
            if label_re.clicked() {
                crate::app::set_clipboard_string(
                    &mut app.clipboard,
                    msg,
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
