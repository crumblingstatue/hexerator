use {
    crate::{gui::Gui, shell::msg_if_fail},
    egui::{Button, Ui},
    gamedebug_core::{IMMEDIATE, PERSISTENT},
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
    if ui.add(Button::new("Debug panel...").shortcut_text("F12")).clicked() {
        ui.close_menu();
        IMMEDIATE.toggle();
        PERSISTENT.toggle();
    }
    ui.separator();
    if ui.button("About Hexerator...").clicked() {
        gui.win.about.open.toggle();
        ui.close_menu();
    }
    ui.separator();
    ui.menu_button("Debug", |ui| {
        #[expect(clippy::panic)]
        if ui.button("Simulate panic (crash hexerator)").clicked() {
            panic!("User induced panic!");
        }
    });
}
