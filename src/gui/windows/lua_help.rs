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
}

impl LuaHelpWindow {
    pub fn ui(ui: &mut egui::Ui, _gui: &mut Gui, _app: &mut App) {
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
                    add_bookmark
                    ] $* {
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
