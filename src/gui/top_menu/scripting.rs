use crate::{
    app::App,
    gui::{
        dialogs::{LuaExecuteDialog, LuaFillDialog},
        Gui,
    },
};

pub fn ui(ui: &mut egui::Ui, gui: &mut Gui, _app: &mut App) {
    if ui.button("Execute script...").clicked() {
        Gui::add_dialog(&mut gui.dialogs, LuaExecuteDialog::default());
        ui.close_menu();
    }
    if ui.button("Lua fill...").clicked() {
        Gui::add_dialog(&mut gui.dialogs, LuaFillDialog::default());
        ui.close_menu();
    }
}
