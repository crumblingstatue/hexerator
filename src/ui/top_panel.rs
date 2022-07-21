use egui_sfml::egui::{self, ComboBox, DragValue, Layout, Ui};
use rand::{thread_rng, RngCore};
use sfml::window::clipboard;

use crate::{
    app::App, color::ColorMethod, damage_region::DamageRegion, msg_if_fail, msg_warn,
    region::Region, slice_ext::SliceExt, source::Source, ui::Dialog,
};

pub fn ui(ui: &mut Ui, app: &mut App) {
    ui.horizontal(|ui| {
        ui.menu_button("File", |ui| {
            if ui.button("Open").clicked() {
                if let Some(file) = rfd::FileDialog::new().pick_file() {
                    msg_if_fail(
                        app.load_file(file, false),
                        "Failed to load file (read-write)",
                    );
                }
                ui.close_menu();
            }
            if ui.button("Open (read only)").clicked() {
                if let Some(file) = rfd::FileDialog::new().pick_file() {
                    msg_if_fail(app.load_file(file, true), "Failed to load file (read-only)");
                }
                ui.close_menu();
            }
            ui.menu_button("Recent", |ui| {
                let mut load = None;
                for entry in app.cfg.recent.iter() {
                    if ui.button(entry.display().to_string()).clicked() {
                        load = Some(entry.clone());
                        ui.close_menu();
                        break;
                    }
                    ui.separator();
                }
                if let Some(path) = load {
                    app.load_file(path, false).unwrap();
                }
            });
            if ui.button("Close").clicked() {
                app.close_file();
                ui.close_menu();
            }
        });
        ui.menu_button("Edit", |ui| {
            if ui.button("Find (ctrl+F)").clicked() {
                app.ui.find_dialog.open ^= true;
                ui.close_menu();
            }
        });
        ui.menu_button("Seek", |ui| {
            let re = ui
                .button("Set cursor to initial position")
                .on_hover_text("Set to --jump argument, 0 otherwise");
            if re.clicked() {
                app.set_cursor_init();
                ui.close_menu();
            }
            if ui.button("Set cursor position").clicked() {
                ui.close_menu();
                #[derive(Debug, Default)]
                struct SetCursorDialog {
                    offset: usize,
                }
                impl Dialog for SetCursorDialog {
                    fn title(&self) -> &str {
                        "Set cursor"
                    }

                    fn ui(&mut self, ui: &mut egui::Ui, app: &mut App) -> bool {
                        ui.horizontal(|ui| {
                            ui.label("Offset");
                            ui.add(DragValue::new(&mut self.offset));
                        });
                        if ui.input().key_pressed(egui::Key::Enter) {
                            app.edit_state.cursor = self.offset;
                            app.center_view_on_offset(self.offset);
                            false
                        } else {
                            true
                        }
                    }
                }
                app.ui.add_dialog(SetCursorDialog::default());
            }
        });
        ui.menu_button("View", |ui| {
            if ui.button("Flash cursor").clicked() {
                app.flash_cursor();
                ui.close_menu();
            }
            if ui.button("Center view on cursor").clicked() {
                app.center_view_on_offset(app.edit_state.cursor);
                app.flash_cursor();
                ui.close_menu();
            }
            if ui.button("Set view offset to cursor").clicked() {
                app.perspective.region.begin = app.edit_state.cursor;
            }
            ui.horizontal(|ui| {
                ui.label("Seek to byte offset");
                let re = ui.text_edit_singleline(&mut app.ui.seek_byte_offset_input);
                if re.lost_focus() && ui.input().key_pressed(egui::Key::Enter) {
                    if let Some(idx) = app.focused_view {
                        app.views[idx].scroll_to_byte_offset(
                            app.ui.seek_byte_offset_input.parse().unwrap_or(0),
                            &app.perspective,
                            app.col_change_lock_x,
                            app.col_change_lock_y,
                        );
                    }
                }
            });
            ui.checkbox(&mut app.col_change_lock_x, "Lock x on column change");
            ui.checkbox(&mut app.col_change_lock_y, "Lock y on column change");
        });
        if ui.button("Regions").clicked() {
            app.ui.regions_window.open ^= true;
        }
        ui.with_layout(Layout::right_to_left(), |ui| {
            match &app.source {
                Some(src) => match src {
                    Source::File(_) => {
                        match &app.args.file {
                            Some(file) => match file.canonicalize() {
                                Ok(path) => ui.label(path.display().to_string()),
                                Err(e) => ui.label(format!("path error: {}", e)),
                            },
                            None => ui.label("File path unknown"),
                        };
                    }
                    Source::Stdin(_) => {
                        ui.label("Standard input");
                    }
                },
                None => {
                    ui.label("No source loaded");
                }
            }
            if app.args.stream {
                if app.stream_end {
                    ui.label("[finished stream]");
                } else {
                    ui.spinner();
                    ui.label("[streaming]");
                }
            }
        });
    });
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
