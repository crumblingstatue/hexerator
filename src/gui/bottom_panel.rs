use egui_sfml::egui::{
    text::LayoutJob, Align, Color32, DragValue, Stroke, TextFormat, TextStyle, Ui,
};
use slotmap::Key;

use crate::{
    app::{interact_mode::InteractMode, App},
    view::ViewportVec,
};

pub fn ui(ui: &mut Ui, app: &mut App, mouse_pos: ViewportVec) {
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
                let per = match app.meta_state.meta.perspectives.get_mut(view.perspective) {
                    Some(per) => per,
                    None => {
                        ui.label("Invalid perspective key");
                        return;
                    }
                };
                ui.label("offset");
                ui.add(DragValue::new(&mut app.meta_state.meta.regions[per.region].region.begin));
                ui.label("columns");
                ui.add(DragValue::new(&mut per.cols));
                let offsets = view.offsets(&app.meta_state.meta.perspectives, &app.meta_state.meta.regions);
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
            app.edit_state.cursor, app.edit_state.cursor
        ));
        if !app.hex_ui.current_layout.is_null() && let Some((offset, _view_idx)) = app.byte_offset_at_pos(mouse_pos.x, mouse_pos.y) {
            ui.label(format!("mouse: {} ({:x})", offset, offset));
        }
    });
}

/// A key "box" and then some text. Like `[F1] View`
fn key_label(ui: &mut Ui, key_text: &str, label_text: &str) -> LayoutJob {
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
            underline: Stroke::none(),
            strikethrough: Stroke::none(),
            valign: Align::Center,
        },
    );
    job.append(
        label_text,
        10.0,
        TextFormat::simple(body_font, style.visuals.widgets.active.fg_stroke.color),
    );
    job
}
