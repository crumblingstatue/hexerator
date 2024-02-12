use {
    super::{dialogs::LuaColorDialog, message_dialog::Icon, top_menu::top_menu, Gui},
    crate::{
        app::App,
        event::EventQueue,
        shell::{msg_fail, msg_if_fail},
        value_color::{self, ColorMethod, Palette},
    },
    anyhow::Context,
    egui_sfml::{
        egui::{self, ComboBox, Layout, Ui},
        sfml::graphics::{Font, Image},
    },
};

pub fn ui(ui: &mut Ui, gui: &mut Gui, app: &mut App, font: &Font, events: &EventQueue) {
    top_menu(ui, gui, app, font, events);
    ui.horizontal(|ui| {
        if app.hex_ui.select_a.is_some() || app.hex_ui.select_b.is_some() {
            ui.label("Selection");
        }
        if let Some(a) = app.hex_ui.select_a {
            ui.label(format!("a: {a}"));
        }
        if let Some(b) = app.hex_ui.select_b {
            ui.label(format!("b: {b}"));
        }
        if let Some(sel) = app.hex_ui.selection()
            && let Some(view_key) = app.hex_ui.focused_view
        {
            let view = &app.meta_state.meta.views[view_key].view;
            let (rows, rem) =
                app.meta_state.meta.low.perspectives[view.perspective].region_row_span(sel);
            ui.label(format!(
                "{rows} rows * {} cols + {rem} = {}",
                app.meta_state.meta.low.perspectives[view.perspective].cols,
                sel.len()
            ));
        }
        if !gui.highlight_set.is_empty() {
            ui.label(format!("{} bytes highlighted", gui.highlight_set.len()));
            if ui.button("Clear").clicked() {
                gui.highlight_set.clear();
            }
        }
        if let Some(view_key) = app.hex_ui.focused_view {
            let presentation = &mut app.meta_state.meta.views[view_key].view.presentation;
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
                            ColorMethod::Pure,
                            ColorMethod::Pure.name(),
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
                            presentation.color_method = ColorMethod::Custom(Box::new(Palette(arr)));
                        }
                    });
                ui.color_edit_button_rgb(&mut app.preferences.bg_color);
                ui.label("Bg color");
                if let ColorMethod::Custom(arr) = &mut presentation.color_method {
                    let Some(&byte) = app.data.get(app.edit_state.cursor) else {
                        return;
                    };
                    let col = &mut arr.0[byte as usize];
                    ui.color_edit_button_srgb(col);
                    ui.label("Byte color");
                    if ui
                        .button("#")
                        .on_hover_text("From hex code on clipboard")
                        .clicked()
                    {
                        match color_from_hexcode(&crate::app::get_clipboard_string(
                            &mut app.clipboard,
                            &mut gui.msg_dialog,
                        )) {
                            Ok(new) => *col = new,
                            Err(e) => {
                                gui.msg_dialog
                                    .open(Icon::Error, "Color parse error", e.to_string())
                            }
                        }
                    }
                    if ui.button("Lua").on_hover_text("From lua script").clicked() {
                        Gui::add_dialog(&mut gui.dialogs, LuaColorDialog::default());
                    }
                    if ui.button("Save").clicked() {
                        if let Some(path) = rfd::FileDialog::new().save_file() {
                            msg_if_fail(
                                value_color::save_palette(arr, &path),
                                "Failed to save pal",
                                &mut gui.msg_dialog,
                            );
                        }
                    }
                    if ui.button("Load").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_file() {
                            match value_color::load_palette(&path) {
                                Ok(pal) => *arr = Box::new(pal),
                                Err(e) => msg_fail(&e, "Failed to load pal", &mut gui.msg_dialog),
                            }
                        }
                    }
                    let tooltip = "\
                    From image file.\n\
                    \n\
                    Pixel by pixel, the image's colors will become the byte colors.
                    ";
                    if ui
                        .add_enabled(app.hex_ui.selection().is_some(), egui::Button::new("img"))
                        .on_hover_text(tooltip)
                        .clicked()
                    {
                        let Some(img_path) = rfd::FileDialog::new().pick_file() else {
                            return;
                        };
                        let result: anyhow::Result<()> = try {
                            let img = Image::from_file(
                                img_path
                                    .to_str()
                                    .context("Failed to convert path to utf-8")?,
                            )
                            .context("Failed to load image")?;
                            let size = img.size();
                            let sel = app.hex_ui.selection().context("Missing app selection")?;
                            let mut i = 0;
                            for y in 0..size.y {
                                for x in 0..size.x {
                                    let color = unsafe { img.pixel_at(x, y) };
                                    let byte = app.data[sel.begin + i];
                                    arr.0[byte as usize] = [color.r, color.g, color.b];
                                    i += 1;
                                }
                            }
                        };
                        msg_if_fail(
                            result,
                            "Failed to load palette from reference image",
                            &mut gui.msg_dialog,
                        );
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
