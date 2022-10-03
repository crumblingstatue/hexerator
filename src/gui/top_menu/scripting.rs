use crate::{
    app::App,
    gui::{
        dialogs::{LuaExecuteDialog, LuaFillDialog},
        Gui,
    },
};

pub fn ui(ui: &mut egui::Ui, gui: &mut Gui, _app: &mut App) {
    if ui.button("Execute script...").clicked() {
        gui.add_dialog(LuaExecuteDialog::default());
    }
    if ui.button("Lua fill...").clicked() {
        gui.add_dialog(LuaFillDialog::default());
        ui.close_menu();
    }
}
