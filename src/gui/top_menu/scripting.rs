use {
    crate::{
        app::App,
        gui::{dialogs::LuaExecuteDialog, Gui},
        shell::msg_if_fail,
    },
    mlua::Lua,
};

pub fn ui(
    ui: &mut egui::Ui,
    gui: &mut Gui,
    app: &mut App,
    lua: &Lua,
    font_size: u16,
    line_spacing: u16,
) {
    if ui.button("🖹 Lua editor").clicked() {
        Gui::add_dialog(&mut gui.dialogs, LuaExecuteDialog::default());
        ui.close_menu();
    }
    if ui.button("📃 Script manager").clicked() {
        gui.win.script_manager.open.toggle();
        ui.close_menu();
    }
    if ui.button("🖳 Quick eval window").clicked() {
        gui.win.lua_console.open.toggle();
        ui.close_menu();
    }
    if ui.button("👁 New watch window").clicked() {
        gui.win.add_lua_watch_window();
        ui.close_menu();
    }
    if ui.button("？ Hexerator Lua API").clicked() {
        gui.win.lua_help.open.toggle();
        ui.close_menu();
    }
    ui.separator();
    let mut scripts = std::mem::take(&mut app.meta_state.meta.scripts);
    for (key, script) in scripts.iter() {
        if ui.button(&script.name).clicked() {
            ui.close_menu();
            let result = crate::scripting::exec_lua(
                lua,
                &script.content,
                app,
                gui,
                "",
                Some(key),
                font_size,
                line_spacing,
            );
            msg_if_fail(result, "Failed to execute script", &mut gui.msg_dialog);
        }
    }
    std::mem::swap(&mut app.meta_state.meta.scripts, &mut scripts);
}
