use egui_sfml::egui::{self, Layout};
use rand::{thread_rng, RngCore};
use sfml::{graphics::Font, window::clipboard};

use crate::{
    app::App, damage_region::DamageRegion, msg_if_fail, msg_info, source::Source, ui::Dialog,
};

pub fn top_menu(ui: &mut egui::Ui, app: &mut App, window_height: i16, font: &Font) {
    ui.horizontal(|ui| {
        ui.menu_button("File", |ui| {
            if ui.button("Open").clicked() {
                if let Some(file) = rfd::FileDialog::new().pick_file() {
                    msg_if_fail(
                        app.load_file(file, false, window_height, font),
                        "Failed to load file (read-write)",
                    );
                }
                ui.close_menu();
            }
            if ui.button("Open (read only)").clicked() {
                if let Some(file) = rfd::FileDialog::new().pick_file() {
                    msg_if_fail(
                        app.load_file(file, true, window_height, font),
                        "Failed to load file (read-only)",
                    );
                }
                ui.close_menu();
            }
            ui.menu_button("Recent", |ui| {
                let mut load = None;
                app.cfg.recent.retain(|entry| {
                    let mut retain = true;
                    ui.horizontal(|ui| {
                        if ui
                            .button(
                                entry
                                    .file
                                    .as_ref()
                                    .map_or_else(|| String::from("Unnamed file"), |path| path.display().to_string())
                            )
                            .clicked()
                        {
                            load = Some(entry.clone());
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("🗑").clicked() {
                            retain = false;
                        }
                    });
                    ui.separator();
                    retain
                });
                if let Some(args) = load {
                    msg_if_fail(
                        app.load_file_args(args, window_height, font),
                        "Failed to load file",
                    );
                }
                ui.separator();
                let mut cap = app.cfg.recent.capacity();
                if ui.add(egui::DragValue::new(&mut cap).prefix("list capacity: ")).changed() {
                    app.cfg.recent.set_capacity(cap);
                }

            });
            ui.separator();
            if ui
                .add_enabled(
                    !app.args.read_only && app.dirty_region.is_some(),
                    egui::Button::new("Save (ctrl+S)"),
                )
                .clicked()
            {
                msg_if_fail(app.save(), "Failed to save");
                ui.close_menu();
            }
            if ui.add(egui::Button::new("Reload (ctrl+R)")).clicked() {
                msg_if_fail(app.reload(), "Failed to reload");
                ui.close_menu();
            }
            ui.checkbox(&mut app.auto_reload, "Auto reload");
            ui.horizontal(|ui| {
                ui.label("Auto reload interval");
                ui.add(egui::DragValue::new(&mut app.auto_reload_interval_ms).suffix("ms"));
            });
            ui.checkbox(&mut app.preferences.auto_save, "Auto save")
                .on_hover_text("Save every time an editing action is finished");
            ui.separator();
            if ui.button("Create backup").clicked() {
                msg_if_fail(app.create_backup(), "Failed to create backup");
                ui.close_menu();
            }
            if ui.button("Restore backup").clicked() {
                msg_if_fail(app.restore_backup(), "Failed to restore backup");
                ui.close_menu();
            }
            ui.separator();
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
            ui.separator();
            if ui.button("Set select a to cursor").clicked() {
                app.select_a = Some(app.edit_state.cursor);
                ui.close_menu();
            }
            if ui.button("Set select b to cursor").clicked() {
                app.select_b = Some(app.edit_state.cursor);
                ui.close_menu();
            }
            if ui.button("Unselect all").clicked() {
                app.select_a = None;
                app.select_b = None;
                ui.close_menu();
            }
            ui.separator();
            if ui.button("Fill selection with random").clicked() {
                if let Some(sel) = App::selection(&app.select_a, &app.select_b) {
                    let range = sel.begin..=sel.end;
                    thread_rng().fill_bytes(&mut app.data[range.clone()]);
                    app.widen_dirty_region(&DamageRegion::RangeInclusive(range));
                }
                ui.close_menu();
            }
            if ui.button("Copy selection as hex").clicked() {
                if let Some(sel) = App::selection(&app.select_a, &app.select_b) {
                    use std::fmt::Write;
                    let mut s = String::new();
                    for &byte in &app.data[sel.begin..=sel.end] {
                        write!(&mut s, "{:02x} ", byte).unwrap();
                    }
                    clipboard::set_string(s.trim_end());
                }
                ui.close_menu();
            }
            if ui.button("Save selection to file").clicked() {
                if let Some(file_path) = rfd::FileDialog::new().save_file() && let Some(sel) = App::selection(&app.select_a, &app.select_b) {
                    let result = std::fs::write(file_path, &app.data[sel.begin..=sel.end]);
                    msg_if_fail(result, "Failed to save selection to file");
                }
                ui.close_menu();
            }
            ui.separator();
            ui.checkbox(&mut app.preferences.move_edit_cursor, "Move edit cursor")
                .on_hover_text("With the cursor keys in edit mode, move edit cursor by default.\n\
                                Otherwise, block cursor is moved. Can use ctrl+cursor keys for
                                the other behavior.");
            ui.checkbox(&mut app.preferences.quick_edit, "Quick edit")
                .on_hover_text("Immediately apply editing results, instead of having to type a \
                                value to completion or press enter");
            ui.checkbox(&mut app.preferences.sticky_edit, "Sticky edit")
                .on_hover_text("Don't automatically move cursor after editing is finished");
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
                            ui.add(egui::DragValue::new(&mut self.offset));
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
                ui.close_menu();
                app.ui.add_dialog(SetCursorDialog::default());
            }
        });
        ui.menu_button("View", |ui| {
            if ui.button("Configure views...").clicked() {
                app.ui.views_window.open ^= true;
                ui.close_menu();
            }
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
            ui.checkbox(
                &mut app.perspective.flip_row_order,
                "Flip row order (experimental)",
            );
        });
        ui.menu_button("Meta", |ui| {
            if ui.button("Regions").clicked() {
                app.ui.regions_window.open ^= true;
                ui.close_menu();
            }
        });
        ui.menu_button("Analysis", |ui| {
            if ui.button("Determine data mime type under cursor").clicked() {
                let format = tree_magic_mini::from_u8(&app.data[app.edit_state.cursor..]);
                msg_info(format);
                ui.close_menu();
            }
        });
        ui.menu_button("Help", |ui| {
            if ui.button("debug panel (F12)").clicked() {
                ui.close_menu();
                gamedebug_core::toggle();
            }
        });
        ui.with_layout(Layout::right_to_left(), |ui| {
            match &app.source {
                Some(src) => match src {
                    Source::File(_) => {
                        match &app.args.file {
                            Some(file) => ui.label(file.display().to_string()),
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
}
