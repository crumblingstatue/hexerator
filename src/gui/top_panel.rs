use {
    super::{dialogs::LuaColorDialog, message_dialog::Icon, top_menu::top_menu, Gui},
    crate::{
        app::App,
        util::human_size,
        value_color::{ColorMethod, Palette},
    },
    anyhow::Context,
    egui::{ComboBox, Layout, Ui},
    mlua::Lua,
};

pub fn ui(ui: &mut Ui, gui: &mut Gui, app: &mut App, lua: &Lua, font_size: u16, line_spacing: u16) {
    top_menu(ui, gui, app, lua, font_size, line_spacing);
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
            ))
            .on_hover_text(human_size(sel.len()));
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
                    if ui.button("#").on_hover_text("From hex code on clipboard").clicked() {
                        match color_from_hexcode(&crate::app::get_clipboard_string(
                            &mut app.clipboard,
                            &mut gui.msg_dialog,
                        )) {
                            Ok(new) => *col = new,
                            Err(e) => {
                                gui.msg_dialog.open(Icon::Error, "Color parse error", e.to_string())
                            }
                        }
                    }
                    if ui.button("Lua").on_hover_text("From lua script").clicked() {
                        Gui::add_dialog(&mut gui.dialogs, LuaColorDialog::default());
                    }
                    if ui.button("Save").clicked() {
                        gui.fileops.save_palette_for_view(view_key);
                    }
                    if ui.button("Load").clicked() {
                        gui.fileops.load_palette_for_view(view_key);
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
                        gui.fileops.load_palette_from_image_for_view(view_key);
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
#[expect(clippy::unwrap_used)]
fn test_color_from_hexcode() {
    assert_eq!(color_from_hexcode("#ffffff").unwrap(), [255, 255, 255]);
    assert_eq!(color_from_hexcode("ff00ff").unwrap(), [255, 0, 255]);
}
