use {
    crate::{
        app::App,
        gui::{dialogs::TruncateDialog, Gui},
        shell::msg_if_fail,
    },
    egui::Button,
};

pub fn ui(ui: &mut egui::Ui, gui: &mut Gui, app: &mut App) {
    if ui
        .add(Button::new("Find...").shortcut_text("Ctrl+F"))
        .clicked()
    {
        gui.find_dialog.open.toggle();
        ui.close_menu();
    }
    ui.separator();
    match app.hex_ui.selection() {
        Some(sel) => {
            if crate::gui::selection_menu::selection_menu(
                "Selection",
                ui,
                app,
                &mut gui.dialogs,
                &mut gui.msg_dialog,
                &mut gui.regions_window,
                sel,
            ) {
                ui.close_menu();
            }
        }
        None => {
            ui.label("<No selection>");
        }
    }
    if ui
        .add(Button::new("Set select a").shortcut_text("shift+1"))
        .clicked()
    {
        app.hex_ui.select_a = Some(app.edit_state.cursor);
        ui.close_menu();
    }
    if ui
        .add(Button::new("Set select b").shortcut_text("shift+2"))
        .clicked()
    {
        app.hex_ui.select_b = Some(app.edit_state.cursor);
        ui.close_menu();
    }
    if ui
        .add(Button::new("Select all in view").shortcut_text("Ctrl+A"))
        .clicked()
    {
        app.focused_view_select_all();
        ui.close_menu();
    }
    ui.separator();
    if ui.button("External command...").clicked() {
        gui.external_command_window.open.toggle();
        ui.close_menu();
    }
    ui.separator();
    ui.menu_button("Paste at cursor", |ui| {
        if ui.button("Hex text from clipboard").clicked() {
            ui.close_menu();
            let s = crate::app::get_clipboard_string(&mut app.clipboard, &mut gui.msg_dialog);
            let mut cursor = app.edit_state.cursor;
            let result: anyhow::Result<()> = try {
                for hex in s.split_ascii_whitespace() {
                    app.data[cursor] = u8::from_str_radix(hex, 16)?;
                    cursor += 1;
                }
            };
            msg_if_fail(result, "Hex text paste error", &mut gui.msg_dialog);
        }
    });
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
        Gui::add_dialog(
            &mut gui.dialogs,
            TruncateDialog::new(app.data.len(), app.hex_ui.selection()),
        );
        ui.close_menu();
    }
}
