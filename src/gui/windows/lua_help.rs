use {
    crate::{
        app::App,
        gui::{window_open::WindowOpen, Gui},
        scripting::*,
    },
    egui::Color32,
};

#[derive(Default)]
pub struct LuaHelpWindow {
    pub open: WindowOpen,
    pub filter: String,
}

impl LuaHelpWindow {
    pub fn ui(ui: &mut egui::Ui, gui: &mut Gui, _app: &mut App) {
        ui.add(egui::TextEdit::singleline(&mut gui.lua_help_window.filter).hint_text("🔍 Filter"));
        egui::ScrollArea::vertical()
            .max_height(500.0)
            .show(ui, |ui| {
                forr::forr! {$t:ty in [
                    add_region,
                    load_file,
                    bookmark_set_int,
                    region_pattern_fill,
                    find_result_offsets,
                    read_u8,
                    read_u32_le,
                    fill_range,
                    set_dirty_region,
                    save,
                    bookmark_offset,
                    add_bookmark,
                    find_hex_string,
                    focus_cursor,
                    ] $* 'block: {
                        let filter_lower = &gui.lua_help_window.filter.to_ascii_lowercase();
                        if !($t::NAME.to_ascii_lowercase().contains(filter_lower) || $t::HELP.to_ascii_lowercase().contains(filter_lower)) {
                            break 'block;
                        }
                        ui.horizontal(|ui| {
                            ui.style_mut().spacing.item_spacing = egui::vec2(0., 0.);
                            ui.label("hx:");
                            ui.label(egui::RichText::new($t::API_SIG).color(Color32::WHITE).strong());
                        });
                        ui.indent("doc_indent", |ui| {
                            ui.label($t::HELP);
                        });
                }};
            });
    }
}
