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
    egui::Button,
    rand::RngCore as _,
    std::fmt::Write as _,
};

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
        if ui.add(Button::new("Unselect").shortcut_text("Esc")).clicked() {
            app.hex_ui.clear_selections();
            ui.close_menu();
            clicked = true;
        }
        if ui.add(Button::new("Zero fill").shortcut_text("Del")).clicked() {
            app.data.zero_fill_region(sel);
            ui.close_menu();
            clicked = true;
        }
        if ui.button("Pattern fill...").clicked() {
            Gui::add_dialog(gui_dialogs, PatternFillDialog::default());
            ui.close_menu();
            clicked = true;
        }
        if ui.button("Lua fill...").clicked() {
            Gui::add_dialog(gui_dialogs, LuaFillDialog::default());
            ui.close_menu();
            clicked = true;
        }
        if ui.button("Random fill").clicked() {
            for region in app.hex_ui.selected_regions() {
                if let Some(data) = app.data.get_mut(region.to_range()) {
                    rand::rng().fill_bytes(data);
                    app.data.widen_dirty_region(DamageRegion::RangeInclusive(region.to_range()));
                }
            }
            ui.close_menu();
            clicked = true;
        }
        if ui.button("Copy as hex text").clicked() {
            let mut s = String::new();
            for &byte in &app.data[sel.begin..=sel.end] {
                write!(&mut s, "{byte:02x} ").unwrap();
            }
            crate::app::set_clipboard_string(&mut app.clipboard, gui_msg_dialog, s.trim_end());
            ui.close_menu();
            clicked = true;
        }
        if ui.button("Copy as utf-8 text").clicked() {
            let s = String::from_utf8_lossy(&app.data[sel.begin..=sel.end]);
            crate::app::set_clipboard_string(&mut app.clipboard, gui_msg_dialog, &s);
            ui.close_menu();
            clicked = true;
        }
        if ui.button("Add as region").clicked() {
            crate::gui::ops::add_region_from_selection(
                sel,
                &mut app.meta_state,
                gui_regions_window,
            );
            ui.close_menu();
            clicked = true;
        }
        if ui.button("Save to file").clicked() {
            file_ops.save_selection_to_file(sel);
            ui.close_menu();
            clicked = true;
        }
        if ui.button("X86 asm").clicked() {
            Gui::add_dialog(gui_dialogs, X86AsmDialog::new());
            ui.close_menu();
            clicked = true;
        }
    });
    clicked
}
