use {
    crate::{
        app::App,
        gui::{message_dialog::MessageDialog, Dialog},
        value_color::ColorMethod,
    },
    egui,
    egui_sfml::sfml::graphics::Font,
    mlua::{Function, Lua},
};

pub struct LuaColorDialog {
    script: String,
    err_string: String,
    auto_exec: bool,
}

impl Default for LuaColorDialog {
    fn default() -> Self {
        const DEFAULT_SCRIPT: &str =
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/lua/color.lua"));
        Self {
            script: DEFAULT_SCRIPT.into(),
            err_string: String::new(),
            auto_exec: Default::default(),
        }
    }
}

impl Dialog for LuaColorDialog {
    fn title(&self) -> &str {
        "Lua color"
    }

    fn ui(
        &mut self,
        ui: &mut egui::Ui,
        app: &mut App,
        _msg: &mut MessageDialog,
        lua: &Lua,
        _font: &Font,
        _events: &mut crate::event::EventQueue,
    ) -> bool {
        let color_data = match app.hex_ui.focused_view {
            Some(view_key) => {
                let view = &mut app.meta_state.meta.views[view_key].view;
                match &mut view.presentation.color_method {
                    ColorMethod::Custom(color_data) => &mut color_data.0,
                    _ => {
                        ui.label("Please select \"Custom\" as color scheme for the current view");
                        return !ui.button("Close").clicked();
                    }
                }
            }
            None => {
                ui.label("No active view");
                return !ui.button("Close").clicked();
            }
        };
        egui::TextEdit::multiline(&mut self.script)
            .code_editor()
            .desired_width(f32::INFINITY)
            .show(ui);
        ui.horizontal(|ui| {
            if ui.button("Execute").clicked() || self.auto_exec {
                let chunk = lua.load(&self.script);
                let res: mlua::Result<()> = try {
                    let fun = chunk.eval::<Function>()?;
                    for (i, c) in color_data.iter_mut().enumerate() {
                        let rgb: [u8; 3] = fun.call((i,))?;
                        *c = rgb;
                    }
                };
                if let Err(e) = res {
                    self.err_string = e.to_string();
                } else {
                    self.err_string.clear();
                }
            }
            ui.checkbox(&mut self.auto_exec, "Auto execute");
        });
        if !self.err_string.is_empty() {
            ui.label(egui::RichText::new(&self.err_string).color(egui::Color32::RED));
        }
        if ui.button("Close").clicked() {
            return false;
        }
        true
    }
}
