use {
    crate::{
        app::App,
        damage_region::DamageRegion,
        gui::{
            Gui,
            dialogs::{LuaFillDialog, PatternFillDialog, X86AsmDialog},
            file_ops::FileOps,
            message_dialog::MessageDialog,
            windows::RegionsWindow,
        },
    },
    constcat::concat,
    egui::Button,
    egui_phosphor::regular as ic,
    rand::RngCore as _,
    std::fmt::Write as _,
};

const L_UNSELECT: &str = concat!(ic::SELECTION_SLASH, " Unselect");
const L_ZERO_FILL: &str = concat!(ic::NUMBER_SQUARE_ZERO, " Zero fill");
const L_PATTERN_FILL: &str = concat!(ic::BINARY, " Pattern fill...");
const L_LUA_FILL: &str = concat!(ic::MOON, " Lua fill...");
const L_RANDOM_FILL: &str = concat!(ic::SHUFFLE, " Random fill");
const L_COPY_AS_HEX_TEXT: &str = concat!(ic::COPY, " Copy as hex text");
const L_COPY_AS_UTF8: &str = concat!(ic::COPY, " Copy as utf-8 text");
const L_ADD_AS_REGION: &str = concat!(ic::RULER, " Add as region");
const L_SAVE_TO_FILE: &str = concat!(ic::FLOPPY_DISK, " Save to file");
const L_X86_ASM: &str = concat!(ic::PIPE_WRENCH, " X86 asm");

/// Returns whether anything was clicked
pub fn selection_menu(
    title: &str,
    ui: &mut egui::Ui,
    app: &mut App,
    gui_dialogs: &mut crate::gui::Dialogs,
    gui_msg_dialog: &mut MessageDialog,
    gui_regions_window: &mut RegionsWindow,
    sel: crate::meta::region::Region,
    file_ops: &mut FileOps,
) -> bool {
    let mut clicked = false;
    ui.menu_button(title, |ui| {
        if ui.add(Button::new(L_UNSELECT).shortcut_text("Esc")).clicked() {
            app.hex_ui.clear_selections();

            clicked = true;
        }
        if ui.add(Button::new(L_ZERO_FILL).shortcut_text("Del")).clicked() {
            app.data.zero_fill_region(sel);

            clicked = true;
        }
        if ui.button(L_PATTERN_FILL).clicked() {
            Gui::add_dialog(gui_dialogs, PatternFillDialog::default());

            clicked = true;
        }
        if ui.button(L_LUA_FILL).clicked() {
            Gui::add_dialog(gui_dialogs, LuaFillDialog::default());

            clicked = true;
        }
        if ui.button(L_RANDOM_FILL).clicked() {
            for region in app.hex_ui.selected_regions() {
                if let Some(data) = app.data.get_mut(region.to_range()) {
                    rand::rng().fill_bytes(data);
                    app.data.widen_dirty_region(DamageRegion::RangeInclusive(region.to_range()));
                }
            }

            clicked = true;
        }
        if ui.button(L_COPY_AS_HEX_TEXT).clicked() {
            let mut s = String::new();
            for &byte in &app.data[sel.begin..=sel.end] {
                write!(&mut s, "{byte:02x} ").unwrap();
            }
            crate::app::set_clipboard_string(&mut app.clipboard, gui_msg_dialog, s.trim_end());

            clicked = true;
        }
        if ui.button(L_COPY_AS_UTF8).clicked() {
            let s = String::from_utf8_lossy(&app.data[sel.begin..=sel.end]);
            crate::app::set_clipboard_string(&mut app.clipboard, gui_msg_dialog, &s);

            clicked = true;
        }
        if ui.button(L_ADD_AS_REGION).clicked() {
            crate::gui::ops::add_region_from_selection(
                sel,
                &mut app.meta_state,
                gui_regions_window,
            );

            clicked = true;
        }
        if ui.button(L_SAVE_TO_FILE).clicked() {
            file_ops.save_selection_to_file(sel);

            clicked = true;
        }
        if ui.button(L_X86_ASM).clicked() {
            Gui::add_dialog(gui_dialogs, X86AsmDialog::new());

            clicked = true;
        }
    });
    clicked
}
