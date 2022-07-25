use egui_sfml::egui::{ComboBox, Layout, Ui};

use crate::{
    app::App, color::ColorMethod, damage_region::DamageRegion, msg_warn, slice_ext::SliceExt,
    view::ViewportScalar,
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
        let end_text = match app.selection {
            Some(sel) => sel.end.to_string(),
            None => "-".to_owned(),
        };
        ui.label(format!("end: {}", end_text));
        if let Some(sel) = &app.selection {
            ui.label(format!("Size: {}", sel.len()));
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
