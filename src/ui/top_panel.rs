use anyhow::Context;
use egui_sfml::egui::{self, ComboBox, Layout, Ui};
use egui_sfml::sfml::graphics::{Font, Image};

use crate::{
    app::App,
    color::{self, ColorMethod},
    shell::{msg_fail, msg_if_fail, msg_warn},
};

use super::top_menu::top_menu;

pub fn ui(ui: &mut Ui, app: &mut App, font: &Font) {
    top_menu(ui, app, font);
    ui.horizontal(|ui| {
        if app.select_a.is_some() || app.select_b.is_some() {
            ui.label("Selection");
        }
        if let Some(a) = app.select_a {
            ui.label(format!("a: {}", a));
        }
        if let Some(b) = app.select_b {
            ui.label(format!("b: {}", b));
        }
        if let Some(sel) = App::selection(&app.select_a, &app.select_b) && let Some(view_idx) = app.focused_view {
            let view = &app.named_views[view_idx].view;
            let (rows, rem) = app.perspectives[view.perspective].region_row_span(sel);
            ui.label(format!(
                "{rows} rows * {} cols + {rem} = {}",
                app.perspectives[view.perspective].cols,
                sel.len()
            ));
        }
        if let Some(view_idx) = app.focused_view {
            let presentation = &mut app.named_views[view_idx].view.presentation;
            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                ui.checkbox(&mut presentation.invert_color, "invert");
                ComboBox::new("color_combo", "Color")
                    .selected_text(presentation.color_method.name())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut presentation.color_method,
                            ColorMethod::Default,
                            ColorMethod::Default.name(),
                        );
                        ui.selectable_value(
                            &mut presentation.color_method,
                            ColorMethod::Mono,
                            ColorMethod::Mono.name(),
                        );
                        ui.selectable_value(
                            &mut presentation.color_method,
                            ColorMethod::Rgb332,
                            ColorMethod::Rgb332.name(),
                        );
                        ui.selectable_value(
                            &mut presentation.color_method,
                            ColorMethod::Vga13h,
                            ColorMethod::Vga13h.name(),
                        );
                        ui.selectable_value(
                            &mut presentation.color_method,
                            ColorMethod::Grayscale,
                            ColorMethod::Grayscale.name(),
                        );
                        if ui
                            .selectable_label(
                                matches!(&presentation.color_method, ColorMethod::Custom(..)),
                                "custom",
                            )
                            .clicked()
                        {
                            #[expect(
                                clippy::cast_possible_truncation,
                                reason = "The array is 256 elements long"
                            )]
                            let arr = std::array::from_fn(|i| {
                                let c = presentation
                                    .color_method
                                    .byte_color(i as u8, presentation.invert_color);
                                [c.r, c.g, c.b]
                            });
                            presentation.color_method = ColorMethod::Custom(Box::new(arr));
                        }
                    });
                ui.color_edit_button_rgb(&mut app.bg_color);
                ui.label("Bg color");
                if let ColorMethod::Custom(arr) = &mut presentation.color_method {
                    let col = &mut arr[app.data[app.edit_state.cursor] as usize];
                    ui.color_edit_button_srgb(col);
                    ui.label("Byte color");
                    if ui
                        .button("#")
                        .on_hover_text("From hex code on clipboard")
                        .clicked()
                    {
                        match color_from_hexcode(&egui_sfml::sfml::window::clipboard::get_string()) {
                            Ok(new) => *col = new,
                            Err(e) => msg_warn(&format!("Color parse error: {}", e)),
                        }
                    }
                    if ui.button("Save").clicked() {
                        if let Some(path) = rfd::FileDialog::new().save_file() {
                            msg_if_fail(color::save_palette(arr, &path), "Failed to save pal");
                        }
                    }
                    if ui.button("Load").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_file() {
                            match color::load_palette(&path) {
                                Ok(pal) => *arr = Box::new(pal),
                                Err(e) => msg_fail(&e, "Failed to load pal"),
                            }
                        }
                    }
                    let tooltip = "\
                    From image file.\n\
                    \n\
                    Pixel by pixel, the image's colors will become the byte colors.
                    ";
                    if ui
                        .add_enabled(
                            App::selection(&app.select_a, &app.select_b).is_some(),
                            egui::Button::new("img"),
                        )
                        .on_hover_text(tooltip)
                        .clicked()
                    {
                        let Some(img_path) = rfd::FileDialog::new().pick_file() else { return };
                        let result: anyhow::Result<()> = try {
                            let img = Image::from_file(
                                img_path
                                    .to_str()
                                    .context("Failed to convert path to utf-8")?,
                            )
                            .context("Failed to load image")?;
                            let size = img.size();
                            let sel = App::selection(&app.select_a, &app.select_b)
                                .context("Missing app selection")?;
                            let mut i = 0;
                            for y in 0..size.y {
                                for x in 0..size.x {
                                    let color = unsafe { img.pixel_at(x, y) };
                                    let byte = app.data[sel.begin + i];
                                    arr[byte as usize] = [color.r, color.g, color.b];
                                    i += 1;
                                }
                            }
                        };
                        msg_if_fail(result, "Failed to load palette from reference image");
                    }
                }
            });
        }
    });
}

fn color_from_hexcode(mut src: &str) -> anyhow::Result<[u8; 3]> {
    let mut out = [0; 3];
    src = src.trim_start_matches('#');
    for (i, byte) in out.iter_mut().enumerate() {
        let src_idx = i * 2;
        *byte = u8::from_str_radix(src.get(src_idx..src_idx + 2).context("Indexing error")?, 16)?;
    }
    Ok(out)
}

#[test]
#[allow(clippy::unwrap_used)]
fn test_color_from_hexcode() {
    assert_eq!(color_from_hexcode("#ffffff").unwrap(), [255, 255, 255]);
    assert_eq!(color_from_hexcode("ff00ff").unwrap(), [255, 0, 255]);
}
