use {
    crate::{
        app::App,
        gui::{dialogs::LuaExecuteDialog, windows::LuaWatchWindow, Gui},
        shell::msg_if_fail,
    },
    egui_sfml::sfml::graphics::Font,
    mlua::Lua,
};

pub fn ui(ui: &mut egui::Ui, gui: &mut Gui, app: &mut App, lua: &Lua, font: &Font) {
    if ui.button("🖳 Lua console").clicked() {
        gui.lua_console_window.open.toggle();
        ui.close_menu();
    }
    if ui.button("🖹 Execute script...").clicked() {
        Gui::add_dialog(&mut gui.dialogs, LuaExecuteDialog::default());
        ui.close_menu();
    }
    if ui.button("📃 Script manager").clicked() {
        gui.script_manager_window.open.toggle();
        ui.close_menu();
    }
    if ui.button("New watch window").clicked() {
        gui.lua_watch_windows.push(LuaWatchWindow::default());
        ui.close_menu();
    }
    if ui.button("？ Lua help").clicked() {
        gui.lua_help_window.open.toggle();
        ui.close_menu();
    }
    ui.separator();
    let mut scripts = std::mem::take(&mut app.meta_state.meta.scripts);
    for (key, script) in scripts.iter() {
        if ui.button(&script.name).clicked() {
            ui.close_menu();
            let result =
                crate::scripting::exec_lua(lua, &script.content, app, gui, font, "", Some(key));
            msg_if_fail(result, "Failed to execute script", &mut gui.msg_dialog);
        }
    }
    std::mem::swap(&mut app.meta_state.meta.scripts, &mut scripts);
}
