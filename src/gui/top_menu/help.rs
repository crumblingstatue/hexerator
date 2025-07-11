use {
    crate::{gui::Gui, shell::msg_if_fail},
    constcat::concat,
    egui::{Button, Ui},
    egui_phosphor::regular as ic,
    gamedebug_core::{IMMEDIATE, PERSISTENT},
};

const L_HEXERATOR_BOOK: &str = concat!(ic::BOOK_OPEN_TEXT, " Hexerator book");
const L_DEBUG_PANEL: &str = concat!(ic::BUG, " Debug panel...");
const L_ABOUT: &str = concat!(ic::QUESTION, " About Hexerator...");

pub fn ui(ui: &mut Ui, gui: &mut Gui) {
    if ui.button(L_HEXERATOR_BOOK).clicked() {
        msg_if_fail(
            open::that(crate::gui::BOOK_URL),
            "Failed to open help",
            &mut gui.msg_dialog,
        );
    }
    if ui.add(Button::new(L_DEBUG_PANEL).shortcut_text("F12")).clicked() {
        IMMEDIATE.toggle();
        PERSISTENT.toggle();
    }
    ui.separator();
    if ui.button(L_ABOUT).clicked() {
        gui.win.about.open.toggle();
    }
}
