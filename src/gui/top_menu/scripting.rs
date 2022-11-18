use crate::{
    app::App,
    gui::{dialogs::LuaExecuteDialog, Gui},
};

pub fn ui(ui: &mut egui::Ui, gui: &mut Gui, _app: &mut App) {
    if ui.button("Execute script...").clicked() {
        Gui::add_dialog(&mut gui.dialogs, LuaExecuteDialog::default());
        ui.close_menu();
    }
}
