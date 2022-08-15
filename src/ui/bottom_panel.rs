use egui_sfml::egui::{text::LayoutJob, Align, DragValue, Stroke, TextFormat, TextStyle, Ui};

use crate::app::{interact_mode::InteractMode, App};

pub fn ui(ui: &mut Ui, app: &mut App) {
    ui.horizontal(|ui| {
        let job = key_label(ui, "F1", "View");
        if ui
            .selectable_label(app.interact_mode == InteractMode::View, job)
            .clicked()
        {
            app.interact_mode = InteractMode::View;
        }
        let job = key_label(ui, "F2", "Edit");
        if ui
            .selectable_label(app.interact_mode == InteractMode::Edit, job)
            .clicked()
        {
            app.interact_mode = InteractMode::Edit;
        }
        ui.separator();
        match app.interact_mode {
            InteractMode::View => {
                ui.label("offset");
                ui.add(DragValue::new(&mut app.perspective.region.begin));
                ui.label("columns");
                ui.add(DragValue::new(&mut app.perspective.cols));
                let data_len = app.data.len();
                if data_len != 0 {
                    if let Some(idx) = app.focused_view {
                        let offsets = app.named_views[idx].view.offsets(&app.perspective);
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
            }
            InteractMode::Edit => 'edit: {
                if app.data.is_empty() {
                    break 'edit;
                }
                ui.label(format!("cursor: {}", app.edit_state.cursor));
                ui.separator();
            }
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
