use egui_sfml::egui::{self, Layout};
use egui_sfml::sfml::{graphics::Font, window::clipboard};
use rand::{thread_rng, RngCore};

use crate::{
    app::{col_change_impl_view_perspective, App},
    damage_region::DamageRegion,
    shell::{msg_if_fail, msg_info},
    source::SourceProvider,
};

use super::{
    dialogs::{AutoSaveReloadDialog, PatternFillDialog, SetCursorDialog},
    util::{button_with_shortcut, ButtonWithShortcut},
};

pub fn top_menu(ui: &mut egui::Ui, app: &mut App, font: &Font) {
    ui.horizontal(|ui| {
        ui.menu_button("File", |ui| {
            if button_with_shortcut(ui, "Open...", "Ctrl+O").clicked() {
                crate::shell::open_file(app, font);
                ui.close_menu();
            }
            if ui.button("Open (read only)...").clicked() {
                if let Some(file) = rfd::FileDialog::new().pick_file() {
                    msg_if_fail(
                        app.load_file(file, true,  font),
                        "Failed to load file (read-only)",
                    );
                }
                ui.close_menu();
            }
            ui.checkbox(&mut app.preferences.keep_meta, "Keep metadata").on_hover_text("Keep metadata when loading a new file");
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
                                    .map(|path| path.display().to_string())
                                    .unwrap_or_else(|| String::from("Unnamed file")),
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
                        app.load_file_args(args,font),
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
                    app.source.is_some_and(|src| src.attr.permissions.write) && app.dirty_region.is_some(),
                    ButtonWithShortcut("Save", "Ctrl+S"),
                )
                .clicked()
            {
                msg_if_fail(app.save(), "Failed to save");
                ui.close_menu();
            }
            if button_with_shortcut(ui, "Reload", "Ctrl+R").clicked() {
                msg_if_fail(app.reload(), "Failed to reload");
                ui.close_menu();
            }
            if ui.button("Auto save/reload...").clicked() {
                ui.close_menu();
                app.ui.add_dialog(AutoSaveReloadDialog);
            }
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
            if button_with_shortcut(ui, "Close", "Ctrl+W").clicked() {
                app.close_file();
                ui.close_menu();
            }
        });
        ui.menu_button("Edit", |ui| {
            if button_with_shortcut(ui, "Find...", "Ctrl+F").clicked() {
                app.ui.find_dialog.open ^= true;
                ui.close_menu();
            }
            ui.separator();
            if button_with_shortcut(ui, "Set select a", "shift+1").clicked() {
                app.select_a = Some(app.edit_state.cursor);
                ui.close_menu();
            }
            if button_with_shortcut(ui, "Set select b", "shift+2").clicked() {
                app.select_b = Some(app.edit_state.cursor);
                ui.close_menu();
            }
            if button_with_shortcut(ui, "Unselect all", "Esc").clicked() {
                app.select_a = None;
                app.select_b = None;
                ui.close_menu();
            }
            ui.separator();
            if ui.button("Pattern fill...").clicked() {
                app.ui.add_dialog(PatternFillDialog::default());
                ui.close_menu();
            }
            if ui.button("Random fill").clicked() {
                if let Some(sel) = App::selection(&app.select_a, &app.select_b) {
                    let range = sel.begin..=sel.end;
                    thread_rng().fill_bytes(&mut app.data[range.clone()]);
                    app.widen_dirty_region(DamageRegion::RangeInclusive(range));
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
        ui.menu_button("Cursor", |ui| {
            let re = ui
                .button("Reset")
                .on_hover_text("Set to initial position.\n\
                                This will be --jump argument, if one was provided, 0 otherwise");
            if re.clicked() {
                app.set_cursor_init();
                ui.close_menu();
            }
            if button_with_shortcut(ui, "Jump...", "Ctrl+J").clicked() {
                ui.close_menu();
                app.ui.add_dialog(SetCursorDialog::default());
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
        });
        ui.menu_button("View", |ui| {
            if button_with_shortcut(ui, "View configuration...", "F5").clicked() {
                app.ui.views_window.open.toggle();
                ui.close_menu();
            }
            if ui.checkbox(&mut app.auto_view_layout, "Auto view layout").clicked() {
                ui.close_menu();
            }
            ui.horizontal(|ui| {
                ui.label("Seek to byte offset");
                let re = ui.text_edit_singleline(&mut app.ui.seek_byte_offset_input);
                if re.lost_focus() && ui.input().key_pressed(egui::Key::Enter) {
                    if let Some(idx) = app.focused_view {
                        app.named_views[idx].view.scroll_to_byte_offset(
                            app.ui.seek_byte_offset_input.parse().unwrap_or(0),
                            &app.perspectives,
                            app.col_change_lock_x,
                            app.col_change_lock_y,
                        );
                    }
                }
            });
            ui.checkbox(&mut app.col_change_lock_x, "Lock x on column change");
            ui.checkbox(&mut app.col_change_lock_y, "Lock y on column change");
        });
        ui.menu_button("Perspective", |ui| {
            let Some(view_idx) = app.focused_view else { return };
            let view = &mut app.named_views[view_idx].view;
            if button_with_shortcut(ui, "Perspectives...", "F6").clicked() {
                app.ui.perspectives_window.open.toggle();
                ui.close_menu();
            }
            if ui.button("Set offset to cursor").clicked() {
                app.regions[app.perspectives[view.perspective].region].region.begin = app.edit_state.cursor;
                ui.close_menu();
            }
            if ui.button("Fill focused view").on_hover_text("Make the column count as big as the active view can fit").clicked() {
                ui.close_menu();
                    view.scroll_offset.pix_xoff = 0;
                    view.scroll_offset.col = 0;
                    #[expect(clippy::cast_sign_loss, reason = "columns is never negative")]
                    {
                        let cols = view.cols() as usize;
                        col_change_impl_view_perspective(
                            view,
                            &mut app.perspectives,
                            &app.regions,
                            |c| *c = cols,
                            app.col_change_lock_x,
                            app.col_change_lock_y
                        );
                    }
            }
            if ui.checkbox(
                &mut app.perspectives[view.perspective].flip_row_order,
                "Flip row order (experimental)",
            ).clicked() {
                ui.close_menu();
            }
        });
        ui.menu_button("Meta", |ui| {
            if button_with_shortcut(ui, "Regions...", "F7").clicked() {
                app.ui.regions_window.open ^= true;
                ui.close_menu();
            }
            ui.separator();
            ui.checkbox(&mut app.meta_dirty, "Mark as changed")
              .on_hover_text("If marked as changed, the metadata will overwrite the old one on quit");
        });
        ui.menu_button("Analysis", |ui| {
            if ui.button("Determine data mime type under cursor").clicked() {
                let format = tree_magic_mini::from_u8(&app.data[app.edit_state.cursor..]);
                msg_info(format);
                ui.close_menu();
            }
        });
        ui.menu_button("Help", |ui| {
            if ui.button("Help...").clicked() {
                app.ui.help_window.open ^= true;
                ui.close_menu();
            }
            if button_with_shortcut(ui, "Debug panel...", "F12").clicked() {
                ui.close_menu();
                gamedebug_core::toggle();
            }
        });
        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
            match &app.source {
                Some(src) => {
                    match src.provider {
                        SourceProvider::File(_) => {
                            match &app.args.file {
                                Some(file) => ui.label(file.display().to_string()),
                                None => ui.label("File path unknown"),
                            };
                        }
                        SourceProvider::Stdin(_) => {
                            ui.label("Standard input");
                        }
                    }
                    if src.attr.stream {
                        if src.state.stream_end {
                            ui.label("[finished stream]");
                        } else {
                            ui.spinner();
                            ui.label("[streaming]");
                        }
                    }
                }
                None => {
                    ui.label("No source loaded");
                }
            }
        });
    });
}
