mod find_dialog;
pub mod inspect_panel;

use egui_sfml::{
    egui::{
        self, Button, ComboBox, DragValue, Layout, ScrollArea, TextEdit, TopBottomPanel, Window,
    },
    SfEgui,
};
use gamedebug_core::{Info, PerEntry, IMMEDIATE, PERSISTENT};
use rand::{thread_rng, RngCore};
use sfml::{system::Vector2i, window::clipboard};

use crate::{
    app::{interact_mode::InteractMode, App},
    color::ColorMethod,
    damage_region::DamageRegion,
    msg_if_fail, msg_warn,
    region::Region,
    slice_ext::SliceExt,
    source::Source,
};

#[derive(Debug, Default)]
pub struct Ui {
    pub inspect_panel: InspectPanel,
    pub find_dialog: FindDialog,
    pub show_debug_panel: bool,
    pub fill_text: String,
    pub center_offset_input: String,
    pub seek_byte_offset_input: String,
}

use self::{
    find_dialog::FindDialog,
    inspect_panel::{inspect_panel_ui, InspectPanel},
};

#[expect(
    clippy::significant_drop_in_scrutinee,
    reason = "this isn't a useful lint for for loops"
)]
// https://github.com/rust-lang/rust-clippy/issues/8987
pub fn do_egui(sf_egui: &mut SfEgui, app: &mut App, mouse_pos: Vector2i) {
    sf_egui.do_frame(|ctx| {
        let mut open = app.ui.show_debug_panel;
        Window::new("Debug").open(&mut open).show(ctx, |ui| {
            for info in IMMEDIATE.lock().unwrap().iter() {
                if let Info::Msg(msg) = info {
                    ui.label(msg);
                }
            }
            gamedebug_core::clear_immediates();
            ui.separator();
            for PerEntry { frame, info } in PERSISTENT.lock().unwrap().iter() {
                if let Info::Msg(msg) = info {
                    ui.label(format!("{}: {}", frame, msg));
                }
            }
        });
        app.ui.show_debug_panel = open;
        open = app.ui.find_dialog.open;
        Window::new("Find").open(&mut open).show(ctx, |ui| {
            if ui
                .text_edit_singleline(&mut app.ui.find_dialog.input)
                .lost_focus()
                && ui.input().key_pressed(egui::Key::Enter)
            {
                let needle = app.ui.find_dialog.input.parse().unwrap();
                app.ui.find_dialog.result_offsets.clear();
                for (offset, &byte) in app.data.iter().enumerate() {
                    if byte == needle {
                        app.ui.find_dialog.result_offsets.push(offset);
                    }
                }
                if let Some(&off) = app.ui.find_dialog.result_offsets.first() {
                    app.search_focus(off);
                }
            }
            ScrollArea::vertical().max_height(480.).show(ui, |ui| {
                for (i, &off) in app.ui.find_dialog.result_offsets.iter().enumerate() {
                    let re =
                        ui.selectable_label(app.ui.find_dialog.result_cursor == i, off.to_string());
                    if let Some(scroll_off) = app.ui.find_dialog.scroll_to && scroll_off == i {
                        re.scroll_to_me(None);
                        app.ui.find_dialog.scroll_to = None;
                    }
                    if re.clicked() {
                        app.search_focus(off);
                        app.ui.find_dialog.result_cursor = i;
                        break;
                    }
                }
            });
            ui.horizontal(|ui| {
                ui.set_enabled(!app.ui.find_dialog.result_offsets.is_empty());
                if (ui.button("Previous (P)").clicked() || ui.input().key_pressed(egui::Key::P))
                    && app.ui.find_dialog.result_cursor > 0
                {
                    app.ui.find_dialog.result_cursor -= 1;
                    let off = app.ui.find_dialog.result_offsets[app.ui.find_dialog.result_cursor];
                    app.search_focus(off);
                    app.ui.find_dialog.scroll_to = Some(app.ui.find_dialog.result_cursor);
                }
                ui.label((app.ui.find_dialog.result_cursor + 1).to_string());
                if (ui.button("Next (N)").clicked() || ui.input().key_pressed(egui::Key::N))
                    && app.ui.find_dialog.result_cursor + 1
                        < app.ui.find_dialog.result_offsets.len()
                {
                    app.ui.find_dialog.result_cursor += 1;
                    let off = app.ui.find_dialog.result_offsets[app.ui.find_dialog.result_cursor];
                    app.search_focus(off);
                    app.ui.find_dialog.scroll_to = Some(app.ui.find_dialog.result_cursor);
                }
                ui.label(format!(
                    "{} results",
                    app.ui.find_dialog.result_offsets.len()
                ));
            });
        });
        app.ui.find_dialog.open = open;
        TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open").clicked() {
                        if let Some(file) = rfd::FileDialog::new().pick_file() {
                            msg_if_fail(app.load_file(file), "Failed to load file");
                        }
                        ui.close_menu();
                    }
                    if ui.button("Close").clicked() {
                        app.close_file();
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
                        app.view.start_offset = app.edit_state.cursor;
                    }
                    ui.horizontal(|ui| {
                        ui.label("Seek to byte offset");
                        let re = ui.text_edit_singleline(&mut app.ui.seek_byte_offset_input);
                        if re.lost_focus() && ui.input().key_pressed(egui::Key::Enter) {
                            app.set_view_to_byte_offset(
                                app.ui.seek_byte_offset_input.parse().unwrap_or(0),
                            );
                        }
                    });
                    ui.checkbox(&mut app.col_change_lock_x, "Lock x on column change");
                    ui.checkbox(&mut app.col_change_lock_y, "Lock y on column change");
                });
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
                        ui.label("[stream]");
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
                    match app.select_begin {
                        Some(begin) => match &mut app.selection {
                            None => {
                                app.selection = Some(Region {
                                    begin,
                                    end: app.edit_state.cursor,
                                })
                            }
                            Some(sel) => sel.end = app.edit_state.cursor,
                        },
                        None => {}
                    }
                }
                if let Some(sel) = &app.selection {
                    ui.label(format!("Size: {}", sel.size()));
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
        });
        TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .selectable_label(app.interact_mode == InteractMode::View, "View (F1)")
                    .clicked()
                {
                    app.interact_mode = InteractMode::View;
                }
                if ui
                    .selectable_label(app.interact_mode == InteractMode::Edit, "Edit (F2)")
                    .clicked()
                {
                    app.interact_mode = InteractMode::Edit;
                }
                ui.separator();
                match app.interact_mode {
                    InteractMode::View => {
                        ui.label("offset");
                        ui.add(DragValue::new(&mut app.view.start_offset));
                        ui.label("columns");
                        ui.add(DragValue::new(&mut app.view.cols));
                        let data_len = app.data.len();
                        if data_len != 0 {
                            let offsets = app.view_offsets();
                            ui.label(format!(
                                "view offset: row {} col {} byte {} ({:.2}%)",
                                offsets.row,
                                offsets.col,
                                offsets.byte,
                                (offsets.byte as f64 / data_len as f64) * 100.0
                            ));
                            let re = ui.add(
                                TextEdit::singleline(&mut app.ui.center_offset_input)
                                    .hint_text("Center view on offset"),
                            );
                            if re.lost_focus() && ui.input().key_pressed(egui::Key::Enter) {
                                if let Ok(offset) = app.ui.center_offset_input.parse() {
                                    app.center_view_on_offset(offset);
                                }
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
                ui.with_layout(Layout::right_to_left(), |ui| {
                    ui.checkbox(&mut app.ui.show_debug_panel, "debug (F12)");
                    ui.checkbox(&mut app.show_block, "block");
                    ui.checkbox(&mut app.show_text, "text");
                    ui.checkbox(&mut app.show_hex, "hex");
                    ui.separator();
                    if ui.add(Button::new("Reload (ctrl+R)")).clicked() {
                        msg_if_fail(app.reload(), "Failed to reload");
                    }
                    if ui
                        .add_enabled(
                            !app.args.read_only && app.dirty_region.is_some(),
                            Button::new("Save (ctrl+S)"),
                        )
                        .clicked()
                    {
                        msg_if_fail(app.save(), "Failed to save");
                    }
                    ui.separator();
                    if ui.button("Restore").clicked() {
                        msg_if_fail(app.restore_backup(), "Failed to restore backup");
                    }
                    if ui.button("Backup").clicked() {
                        msg_if_fail(app.create_backup(), "Failed to create backup");
                    }
                })
            })
        });
        egui::SidePanel::right("right_panel").show(ctx, |ui| inspect_panel_ui(ui, app, mouse_pos));
    });
}
