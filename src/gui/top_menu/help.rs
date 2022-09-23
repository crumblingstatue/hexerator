use {
    crate::{
        gui::{util::button_with_shortcut, Gui},
        shell::msg_if_fail,
    },
    egui::Ui,
};

pub fn ui(ui: &mut Ui, gui: &mut Gui) {
    if ui.button("Hexerator book").clicked() {
        msg_if_fail(
            open::that("https://crumblingstatue.github.io/hexerator-book/"),
            "Failed to open help",
            &mut gui.msg_dialog,
        );
        ui.close_menu();
    }
    if button_with_shortcut(ui, "Debug panel...", "F12").clicked() {
        ui.close_menu();
        gamedebug_core::toggle();
    }
    ui.separator();
    if ui.button("About Hexerator...").clicked() {
        gui.about_window.open.toggle();
        ui.close_menu();
    }
}
