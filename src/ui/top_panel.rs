use egui_sfml::egui::{ComboBox, Layout, Ui};
use rand::{thread_rng, RngCore};
use sfml::window::clipboard;

use crate::{
    app::App, color::ColorMethod, damage_region::DamageRegion, msg_if_fail, msg_warn,
    region::Region, slice_ext::SliceExt, view::ViewportScalar,
};

use super::top_menu::top_menu;

pub fn ui(ui: &mut Ui, app: &mut App, window_height: ViewportScalar) {
    top_menu(ui, app, window_height);
    ui.horizontal(|ui| {
        let begin_text = match app.select_begin {
            Some(begin) => begin.to_string(),
            None => "-".to_owned(),
        };
        ui.label(format!("Select begin: {}", begin_text));
        if ui.button("set").clicked() {
            match &mut app.selection {
                Some(sel) => sel.begin = app.edit_state.cursor,
                None => app.select_begin = Some(app.edit_state.cursor),
            }
        }
        let end_text = match app.selection {
            Some(sel) => sel.end.to_string(),
            None => "-".to_owned(),
        };
        ui.label(format!("end: {}", end_text));
        if ui.button("set").clicked() {
            if let Some(begin) = app.select_begin {
                match &mut app.selection {
                    None => {
                        app.selection = Some(Region {
                            begin,
                            end: app.edit_state.cursor,
                        })
                    }
                    Some(sel) => sel.end = app.edit_state.cursor,
                }
            }
        }
        if let Some(sel) = &app.selection {
            ui.label(format!("Size: {}", sel.len()));
        }
        if ui.button("deselect").clicked() {
            app.selection = None;
        }
        ui.text_edit_singleline(&mut app.ui.fill_text);
        if ui.button("fill").clicked() {
            if let Some(sel) = app.selection {
                let values: Result<Vec<u8>, _> = app
                    .ui
                    .fill_text
                    .split(' ')
                    .map(|token| u8::from_str_radix(token, 16))
                    .collect();
                match values {
                    Ok(values) => {
                        let range = sel.begin..=sel.end;
                        app.data[range.clone()].pattern_fill(&values);
                        app.widen_dirty_region(DamageRegion::RangeInclusive(range));
                    }
                    Err(e) => {
                        msg_warn(&format!("Fill parse error: {}", e));
                    }
                }
            }
        }
        if ui.button("fill random").clicked() {
            if let Some(sel) = app.selection {
                let range = sel.begin..=sel.end;
                thread_rng().fill_bytes(&mut app.data[range.clone()]);
                app.widen_dirty_region(DamageRegion::RangeInclusive(range));
            }
        }
        if ui.button("copy as hex pattern").clicked() {
            if let Some(sel) = app.selection {
                use std::fmt::Write;
                let mut s = String::new();
                for &byte in &app.data[sel.begin..=sel.end] {
                    write!(&mut s, "{:02x} ", byte).unwrap();
                }
                clipboard::set_string(s.trim_end());
            }
        }
        if ui.button("save to file").clicked() && let Some(file_path) = rfd::FileDialog::new().save_file() && let Some(sel) = app.selection {
            let result = std::fs::write(file_path, &app.data[sel.begin..=sel.end]);
            msg_if_fail(result, "Failed to save selection to file");
        }
        ui.with_layout(Layout::right_to_left(), |ui| {
            ui.checkbox(&mut app.presentation.invert_color, "invert");
            ComboBox::new("color_combo", "Color")
                .selected_text(app.presentation.color_method.name())
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut app.presentation.color_method,
                        ColorMethod::Default,
                        ColorMethod::Default.name(),
                    );
                    ui.selectable_value(
                        &mut app.presentation.color_method,
                        ColorMethod::Mono,
                        ColorMethod::Mono.name(),
                    );
                    ui.selectable_value(
                        &mut app.presentation.color_method,
                        ColorMethod::Rgb332,
                        ColorMethod::Rgb332.name(),
                    );
                    ui.selectable_value(
                        &mut app.presentation.color_method,
                        ColorMethod::Vga13h,
                        ColorMethod::Vga13h.name(),
                    );
                    ui.selectable_value(
                        &mut app.presentation.color_method,
                        ColorMethod::Grayscale,
                        ColorMethod::Grayscale.name(),
                    );
                    ui.selectable_value(
                        &mut app.presentation.color_method,
                        ColorMethod::Aitd,
                        ColorMethod::Aitd.name(),
                    );
                });
            ui.color_edit_button_rgb(&mut app.presentation.bg_color);
            ui.label("Bg color");
        });
    });
}
