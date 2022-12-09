use {
    crate::{
        app::App,
        gui::{dialogs::JumpDialog, Gui},
    },
    egui::Button,
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
    if ui
        .add(Button::new("Jump...").shortcut_text("Ctrl+J"))
        .clicked()
    {
        ui.close_menu();
        Gui::add_dialog(&mut gui.dialogs, JumpDialog::default());
    }
    if ui.button("Flash cursor").clicked() {
        app.preferences.hide_cursor = false;
        app.hex_ui.flash_cursor();
        ui.close_menu();
    }
    if ui.button("Center view on cursor").clicked() {
        app.preferences.hide_cursor = false;
        app.center_view_on_offset(app.edit_state.cursor);
        app.hex_ui.flash_cursor();
        ui.close_menu();
    }
    if ui
        .checkbox(&mut app.preferences.hide_cursor, "Hide cursor")
        .clicked()
    {
        ui.close_menu();
    }
}
