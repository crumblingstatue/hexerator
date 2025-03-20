use {
    crate::{
        app::App,
        gui::{Gui, dialogs::JumpDialog},
    },
    constcat::concat,
    egui::Button,
    egui_phosphor::regular as ic,
};

const L_RESET: &str = concat!(ic::ARROW_U_UP_LEFT, " Reset");
const L_JUMP: &str = concat!(ic::SHARE_FAT, " Jump...");
const L_FLASH_CURSOR: &str = concat!(ic::LIGHTBULB, " Flash cursor");
const L_CENTER_VIEW_ON_CURSOR: &str = concat!(ic::CROSSHAIR, " Center view on cursor");

pub fn ui(ui: &mut egui::Ui, gui: &mut Gui, app: &mut App) {
    let re = ui.button(L_RESET).on_hover_text(
        "Set to initial position.\n\
                        This will be --jump argument, if one was provided, 0 otherwise",
    );
    if re.clicked() {
        app.set_cursor_init();
        ui.close_menu();
    }
    if ui.add(Button::new(L_JUMP).shortcut_text("Ctrl+J")).clicked() {
        ui.close_menu();
        Gui::add_dialog(&mut gui.dialogs, JumpDialog::default());
    }
    if ui.button(L_FLASH_CURSOR).clicked() {
        app.preferences.hide_cursor = false;
        app.hex_ui.flash_cursor();
        ui.close_menu();
    }
    if ui.button(L_CENTER_VIEW_ON_CURSOR).clicked() {
        app.preferences.hide_cursor = false;
        app.center_view_on_offset(app.edit_state.cursor);
        app.hex_ui.flash_cursor();
        ui.close_menu();
    }
    if ui.checkbox(&mut app.preferences.hide_cursor, "Hide cursor").clicked() {
        ui.close_menu();
    }
}
