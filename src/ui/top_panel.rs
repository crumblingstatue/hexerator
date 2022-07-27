use anyhow::Context;
use egui_sfml::egui::{self, ComboBox, Layout, Ui};
use sfml::graphics::Image;

use crate::{
    app::App, color::ColorMethod, damage_region::DamageRegion, msg_if_fail, msg_warn,
    slice_ext::SliceExt, view::ViewportScalar,
};

use super::top_menu::top_menu;

pub fn ui(ui: &mut Ui, app: &mut App, window_height: ViewportScalar) {
    top_menu(ui, app, window_height);
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
        if let Some(sel) = App::selection(&app.select_a, &app.select_b) {
            let (rows, rem) = app.perspective.region_row_span(sel);
            ui.label(format!(
                "{rows} rows * {} cols + {rem} = {}",
                app.perspective.cols,
                sel.len()
            ));
            ui.text_edit_singleline(&mut app.ui.fill_text);
            if ui.button("fill").clicked() {
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
                    if ui
                        .selectable_label(
                            matches!(&app.presentation.color_method, ColorMethod::Custom(..)),
                            "custom",
                        )
                        .clicked()
                    {
                        #[expect(
                            clippy::cast_possible_truncation,
                            reason = "The array is 256 elements long"
                        )]
                        let arr = std::array::from_fn(|i| {
                            let c = app
                                .presentation
                                .color_method
                                .byte_color(i as u8, app.presentation.invert_color);
                            [c.red(), c.green(), c.blue()]
                        });
                        app.presentation.color_method = ColorMethod::Custom(Box::new(arr));
                    }
                });
            ui.color_edit_button_rgb(&mut app.presentation.bg_color);
            ui.label("Bg color");
            if let ColorMethod::Custom(arr) = &mut app.presentation.color_method {
                let col = &mut arr[app.data[app.edit_state.cursor] as usize];
                ui.color_edit_button_srgb(col);
                ui.label("Byte color");
                if ui
                    .button("#")
                    .on_hover_text("From hex code on clipboard")
                    .clicked()
                {
                    match color_from_hexcode(
                        &sfml::window::clipboard::get_string().to_rust_string(),
                    ) {
                        Ok(new) => *col = new,
                        Err(e) => msg_warn(&format!("Color parse error: {}", e)),
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
                                arr[byte as usize] = [color.red(), color.green(), color.blue()];
                                i += 1;
                            }
                        }
                    };
                    msg_if_fail(result, "Failed to load palette from reference image");
                }
            }
        });
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
