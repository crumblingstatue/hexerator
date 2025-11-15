use {
    crate::{
        app::{
            App,
            command::{Cmd, perform_command},
        },
        gui::{Gui, dialogs::TruncateDialog, message_dialog::Icon},
        result_ext::AnyhowConv as _,
        shell::msg_if_fail,
    },
    constcat::concat,
    egui::Button,
    egui_phosphor::regular as ic,
    mlua::Lua,
};

const L_FIND: &str = concat!(ic::MAGNIFYING_GLASS, " Find...");
const L_SELECTION: &str = concat!(ic::SELECTION, " Selection");
const L_SELECT_A: &str = "🅰 Set select a";
const L_SELECT_B: &str = "🅱 Set select b";
const L_SELECT_ALL: &str = concat!(ic::SELECTION_ALL, " Select all in region");
const L_SELECT_ROW: &str = concat!(ic::ARROWS_HORIZONTAL, " Select row");
const L_SELECT_COL: &str = concat!(ic::ARROWS_VERTICAL, " Select column");
const L_EXTERNAL_COMMAND: &str = concat!(ic::TERMINAL_WINDOW, " External command...");
const L_INC_BYTE: &str = concat!(ic::PLUS, " Inc byte(s)");
const L_DEC_BYTE: &str = concat!(ic::MINUS, " Dec byte(s)");
const L_PASTE_AT_CURSOR: &str = concat!(ic::CLIPBOARD_TEXT, " Paste at cursor");
const L_TRUNCATE_EXTEND: &str = concat!(ic::SCISSORS, " Truncate/Extend...");

pub fn ui(
    ui: &mut egui::Ui,
    gui: &mut Gui,
    app: &mut App,
    lua: &Lua,
    font_size: u16,
    line_spacing: u16,
) {
    if ui.add(Button::new(L_FIND).shortcut_text("Ctrl+F")).clicked() {
        gui.win.find.open.toggle();
    }
    ui.separator();
    match app.hex_ui.selection() {
        Some(sel) => {
            if crate::gui::selection_menu::selection_menu(
                L_SELECTION,
                ui,
                app,
                &mut gui.dialogs,
                &mut gui.msg_dialog,
                &mut gui.win.regions,
                sel,
                &mut gui.fileops,
            ) {}
        }
        None => {
            ui.label("<No selection>");
        }
    }
    if ui.add(Button::new(L_SELECT_A).shortcut_text("shift+1")).clicked() {
        app.hex_ui.select_a = Some(app.edit_state.cursor);
    }
    if ui.add(Button::new(L_SELECT_B).shortcut_text("shift+2")).clicked() {
        app.hex_ui.select_b = Some(app.edit_state.cursor);
    }
    if ui.add(Button::new(L_SELECT_ALL).shortcut_text("Ctrl+A")).clicked() {
        app.focused_view_select_all();
    }
    if ui.add(Button::new(L_SELECT_ROW)).clicked() {
        app.focused_view_select_row();
    }
    if ui.add(Button::new(L_SELECT_COL)).clicked() {
        app.focused_view_select_col();
    }
    ui.separator();
    if ui.add(Button::new(L_EXTERNAL_COMMAND).shortcut_text("Ctrl+E")).clicked() {
        gui.win.external_command.open.toggle();
    }
    ui.separator();
    if ui
        .add(Button::new(L_INC_BYTE).shortcut_text("Ctrl+="))
        .on_hover_text("Increase byte(s) of selection or at cursor")
        .clicked()
    {
        app.inc_byte_or_bytes();
    }
    if ui
        .add(Button::new(L_DEC_BYTE).shortcut_text("Ctrl+-"))
        .on_hover_text("Decrease byte(s) of selection or at cursor")
        .clicked()
    {
        app.dec_byte_or_bytes();
    }
    ui.menu_button(L_PASTE_AT_CURSOR, |ui| {
        if ui.button("Hex text from clipboard").clicked() {
            let s = crate::app::get_clipboard_string(&mut app.clipboard, &mut gui.msg_dialog);
            let cursor = app.edit_state.cursor;
            let result = try {
                let bytes = s
                    .split_ascii_whitespace()
                    .map(|s| u8::from_str_radix(s, 16))
                    .collect::<Result<Vec<_>, _>>()
                    .how()?;
                if cursor + bytes.len() < app.data.len() {
                    perform_command(
                        app,
                        Cmd::PasteBytes { at: cursor, bytes },
                        gui,
                        lua,
                        font_size,
                        line_spacing,
                    );
                } else {
                    gui.msg_dialog.open(
                        Icon::Warn,
                        "Prompt",
                        "Paste overflows the document. What do do?",
                    );
                    gui.msg_dialog.custom_button_row_ui(Box::new(move |ui, payload, cmd| {
                        if ui.button("Cancel paste").clicked() {
                            payload.close = true;
                        } else if ui.button("Extend document").clicked() {
                            cmd.push(Cmd::ExtendDocument {
                                new_len: cursor + bytes.len(),
                            });
                            cmd.push(Cmd::PasteBytes {
                                at: cursor,
                                bytes: bytes.clone(),
                            });
                            payload.close = true;
                        } else if ui.button("Shorten paste").clicked() {
                        }
                    }));
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
    ui.checkbox(&mut app.preferences.quick_edit, "Quick edit").on_hover_text(
        "Immediately apply editing results, instead of having to type a \
                        value to completion or press enter",
    );
    ui.checkbox(&mut app.preferences.sticky_edit, "Sticky edit")
        .on_hover_text("Don't automatically move cursor after editing is finished");
    ui.separator();
    if ui.button(L_TRUNCATE_EXTEND).clicked() {
        Gui::add_dialog(
            &mut gui.dialogs,
            TruncateDialog::new(app.data.len(), app.hex_ui.selection()),
        );
    }
}
