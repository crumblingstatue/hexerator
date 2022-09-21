use {
    crate::{
        app::App,
        gui::{dialogs::JumpDialog, util::button_with_shortcut, Gui},
    },
    egui_sfml::egui,
};

pub fn ui(ui: &mut egui::Ui, gui: &mut Gui, app: &mut App) {
    let re = ui.button("Reset").on_hover_text(
        "Set to initial position.\n\
                        This will be --jump argument, if one was provided, 0 otherwise",
    );
    if re.clicked() {
        app.set_cursor_init();
        ui.close_menu();
    }
    if button_with_shortcut(ui, "Jump...", "Ctrl+J").clicked() {
        ui.close_menu();
        gui.add_dialog(JumpDialog::default());
    }
    if ui.button("Flash cursor").clicked() {
        app.hex_ui.flash_cursor();
        ui.close_menu();
    }
    if ui.button("Center view on cursor").clicked() {
        app.center_view_on_offset(app.edit_state.cursor);
        app.hex_ui.flash_cursor();
        ui.close_menu();
    }
}
