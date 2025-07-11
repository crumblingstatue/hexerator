use {
    super::{Gui, dialogs::JumpDialog, egui_ui_ext::EguiResponseExt as _},
    crate::{
        app::{App, interact_mode::InteractMode},
        meta::find_most_specific_region_for_offset,
        shell::msg_if_fail,
        util::human_size,
        view::ViewportVec,
    },
    constcat::concat,
    egui::{Align, Color32, DragValue, Stroke, TextFormat, TextStyle, Ui, text::LayoutJob},
    egui_phosphor::regular as ic,
    slotmap::Key as _,
};

const L_SCROLL: &str = concat!(ic::MOUSE_SCROLL, " scroll");

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
        if data_len != 0
            && let Some(view_key) = app.hex_ui.focused_view
        {
            let view = &mut app.meta_state.meta.views[view_key].view;
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
            let re = ui.button(L_SCROLL);
            if re.clicked() {
                gui.show_quick_scroll_popup ^= true;
            }
            #[expect(
                clippy::cast_precision_loss,
                reason = "Precision is good until 52 bits (more than reasonable)"
            )]
            let mut ratio = offsets.byte as f64 / data_len as f64;
            if gui.show_quick_scroll_popup {
                let avail_w = ui.available_width();
                egui::Window::new("quick_scroll_popup")
                    .resizable(false)
                    .title_bar(false)
                    .fixed_pos(re.rect.right_top())
                    .show(ui.ctx(), |ui| {
                        ui.spacing_mut().slider_width = avail_w * 0.8;
                        let re = ui.add(
                            egui::Slider::new(&mut ratio, 0.0..=1.0)
                                .custom_formatter(|n, _| format!("{:.2}%", n * 100.)),
                        );
                        if re.changed() {
                            // This is used for a rough scroll, so lossy conversion is to be expected
                            #[expect(
                                clippy::cast_possible_truncation,
                                clippy::cast_precision_loss,
                                clippy::cast_sign_loss
                            )]
                            let new_off = (app.data.len() as f64 * ratio) as usize;
                            view.scroll_to_byte_offset(
                                new_off,
                                &app.meta_state.meta.low.perspectives,
                                &app.meta_state.meta.low.regions,
                                false,
                                true,
                            );
                        }
                        ui.horizontal(|ui| {
                            ui.label(human_size(offsets.byte));
                            if ui.button("Close").clicked() {
                                gui.show_quick_scroll_popup = false;
                            }
                        });
                    });
            }
            ui.label(format!(
                "row {} col {} byte {} ({:.2}%)",
                offsets.row,
                offsets.col,
                offsets.byte,
                ratio * 100.0
            ))
            .on_hover_text_deferred(|| human_size(offsets.byte));
        }
        ui.separator();
        let [row, col] = app.row_col_of_cursor().unwrap_or([0, 0]);
        let mut text = egui::RichText::new(format!(
            "cursor: {} ({:x}) [r{row} c{col}]",
            app.edit_state.cursor, app.edit_state.cursor,
        ));
        let out_of_bounds = app.edit_state.cursor >= app.data.len();
        let cursor_end = app.edit_state.cursor == app.data.len().saturating_sub(1);
        let cursor_begin = app.edit_state.cursor == 0;
        if out_of_bounds {
            text = text.color(Color32::RED);
        } else if cursor_end {
            text = text.color(Color32::YELLOW);
        } else if cursor_begin {
            text = text.color(Color32::GREEN);
        }
        let re = ui.label(text);
        re.context_menu(|ui| {
            if ui.button("Copy").clicked() {
                let result = app.clipboard.set_text(app.edit_state.cursor.to_string());
                msg_if_fail(result, "Failed to set clipboard text", &mut gui.msg_dialog);
            }
            if ui.button("Copy absolute").on_hover_text("Hard seek + cursor").clicked() {
                let result = app.clipboard.set_text(
                    (app.edit_state.cursor + app.src_args.hard_seek.unwrap_or(0)).to_string(),
                );
                msg_if_fail(result, "Failed to set clipboard text", &mut gui.msg_dialog);
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
                }
            });
        }
        if !app.hex_ui.current_layout.is_null()
            && let Some((offset, _view_idx)) = app.byte_offset_at_pos(mouse_pos.x, mouse_pos.y)
        {
            let [row, col] = app.row_col_of_byte_pos(offset).unwrap_or([0, 0]);
            ui.label(format!("mouse: {offset} ({offset:x}) [r{row} c{col}]"));
            if let Some(region) =
                find_most_specific_region_for_offset(&app.meta_state.meta.low.regions, offset)
            {
                region_label(ui, &app.meta_state.meta.low.regions[region].name);
            }
        }
        ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
            let mut txt = egui::RichText::new(format!("File size: {}", app.data.len()));
            let truncated = app.data.len() != app.data.orig_data_len;
            if truncated {
                txt = txt.color(Color32::RED);
            }
            let label = egui::Label::new(txt).sense(egui::Sense::click());
            let mut label_re = ui.add(label).on_hover_ui(|ui| {
                ui.label("Click to copy");
                ui.label(format!("Human size: {}", human_size(app.data.len())));
            });
            if truncated {
                label_re = label_re.on_hover_text_deferred(|| {
                    format!("Length changed, orig.: {}", app.data.orig_data_len)
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
        egui::Label::new(egui::RichText::new(format!("[{name}]")).color(Color32::LIGHT_BLUE))
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
