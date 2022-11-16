use {
    crate::{
        app::App,
        damage_region::DamageRegion,
        gui::{
            dialogs::{PatternFillDialog, TruncateDialog},
            util::button_with_shortcut,
            Gui,
        },
        shell::msg_if_fail,
    },
    rand::{thread_rng, RngCore},
    std::fmt::Write,
};

pub fn ui(ui: &mut egui::Ui, gui: &mut Gui, app: &mut App) {
    if button_with_shortcut(ui, "Find...", "Ctrl+F").clicked() {
        gui.find_dialog.open.toggle();
        ui.close_menu();
    }
    ui.separator();
    let no_sel = app.hex_ui.select_a.is_none() || app.hex_ui.select_b.is_none();
    if no_sel {
        ui.label("<No selection>");
    } else {
        ui.menu_button("Selection", |ui| {
            if button_with_shortcut(ui, "Unselect", "Esc").clicked() {
                app.hex_ui.select_a = None;
                app.hex_ui.select_b = None;
                ui.close_menu();
            }
            if ui.button("Pattern fill...").clicked() {
                gui.add_dialog(PatternFillDialog::default());
                ui.close_menu();
            }
            if ui.button("Random fill").clicked() {
                if let Some(sel) = app.hex_ui.selection() {
                    let range = sel.begin..=sel.end;
                    thread_rng().fill_bytes(&mut app.data[range.clone()]);
                    app.edit_state
                        .widen_dirty_region(DamageRegion::RangeInclusive(range));
                }
                ui.close_menu();
            }
            if ui.button("Copy as hex text").clicked() {
                if let Some(sel) = app.hex_ui.selection() {
                    let mut s = String::new();
                    for &byte in &app.data[sel.begin..=sel.end] {
                        write!(&mut s, "{byte:02x} ").unwrap();
                    }
                    ui.output().copied_text = s.trim_end().to_string();
                }
                ui.close_menu();
            }
            if ui.button("Save to file").clicked() {
                if let Some(file_path) = rfd::FileDialog::new().save_file() && let Some(sel) = app.hex_ui.selection() {
                    let result = std::fs::write(file_path, &app.data[sel.begin..=sel.end]);
                    msg_if_fail(result, "Failed to save selection to file", &mut gui.msg_dialog);
                }
                ui.close_menu();
            }
        });
    }
    if button_with_shortcut(ui, "Set select a", "shift+1").clicked() {
        app.hex_ui.select_a = Some(app.edit_state.cursor);
        ui.close_menu();
    }
    if button_with_shortcut(ui, "Set select b", "shift+2").clicked() {
        app.hex_ui.select_b = Some(app.edit_state.cursor);
        ui.close_menu();
    }
    if button_with_shortcut(ui, "Select all in view", "Ctrl+A").clicked() {
        app.focused_view_select_all();
        ui.close_menu();
    }
    ui.separator();
    if ui.button("External command...").clicked() {
        gui.external_command_window.open.toggle();
        ui.close_menu();
    }
    ui.separator();
    ui.checkbox(&mut app.preferences.move_edit_cursor, "Move edit cursor")
        .on_hover_text(
            "With the cursor keys in edit mode, move edit cursor by default.\n\
                        Otherwise, block cursor is moved. Can use ctrl+cursor keys for
                        the other behavior.",
        );
    ui.checkbox(&mut app.preferences.quick_edit, "Quick edit")
        .on_hover_text(
            "Immediately apply editing results, instead of having to type a \
                        value to completion or press enter",
        );
    ui.checkbox(&mut app.preferences.sticky_edit, "Sticky edit")
        .on_hover_text("Don't automatically move cursor after editing is finished");
    ui.separator();
    if ui.button("Truncate/Extend").clicked() {
        gui.add_dialog(TruncateDialog::new(app.data.len(), app.hex_ui.selection()));
        ui.close_menu();
    }
}
