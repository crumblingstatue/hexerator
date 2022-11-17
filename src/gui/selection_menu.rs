use {
    super::{
        dialogs::PatternFillDialog, message_dialog::MessageDialog, regions_window::RegionsWindow,
        util::button_with_shortcut, Gui,
    },
    crate::{app::App, damage_region::DamageRegion, shell::msg_if_fail},
    rand::RngCore,
    std::fmt::Write,
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
) -> bool {
    let mut clicked = false;
    ui.menu_button(title, |ui| {
        if button_with_shortcut(ui, "Unselect", "Esc").clicked() {
            app.hex_ui.select_a = None;
            app.hex_ui.select_b = None;
            ui.close_menu();
            clicked = true;
        }
        if ui.button("Pattern fill...").clicked() {
            Gui::add_dialog(gui_dialogs, PatternFillDialog::default());
            ui.close_menu();
            clicked = true;
        }
        if ui.button("Random fill").clicked() {
            let range = sel.begin..=sel.end;
            rand::thread_rng().fill_bytes(&mut app.data[range.clone()]);
            app.edit_state
                .widen_dirty_region(DamageRegion::RangeInclusive(range));
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
            if let Some(file_path) = rfd::FileDialog::new().save_file() {
                let result = std::fs::write(file_path, &app.data[sel.begin..=sel.end]);
                msg_if_fail(result, "Failed to save selection to file", gui_msg_dialog);
            }
            ui.close_menu();
            clicked = true;
        }
    });
    clicked
}
