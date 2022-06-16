mod bottom_panel;
mod find_dialog;
pub mod inspect_panel;
mod top_panel;

use egui_sfml::{
    egui::{self, ScrollArea, TopBottomPanel, Window},
    SfEgui,
};
use gamedebug_core::{Info, PerEntry, IMMEDIATE, PERSISTENT};
use sfml::system::Vector2i;

use crate::app::App;

#[derive(Debug, Default)]
pub struct Ui {
    pub inspect_panel: InspectPanel,
    pub find_dialog: FindDialog,
    pub show_debug_panel: bool,
    pub fill_text: String,
    pub center_offset_input: String,
    pub seek_byte_offset_input: String,
}

use self::{
    bottom_panel::bottom_panel_ui,
    find_dialog::FindDialog,
    inspect_panel::{inspect_panel_ui, InspectPanel},
    top_panel::top_panel_ui,
};

#[expect(
    clippy::significant_drop_in_scrutinee,
    reason = "this isn't a useful lint for for loops"
)]
// https://github.com/rust-lang/rust-clippy/issues/8987
pub fn do_egui(sf_egui: &mut SfEgui, app: &mut App, mouse_pos: Vector2i) {
    sf_egui.do_frame(|ctx| {
        let mut open = app.ui.show_debug_panel;
        Window::new("Debug").open(&mut open).show(ctx, |ui| {
            for info in IMMEDIATE.lock().unwrap().iter() {
                if let Info::Msg(msg) = info {
                    ui.label(msg);
                }
            }
            gamedebug_core::clear_immediates();
            ui.separator();
            for PerEntry { frame, info } in PERSISTENT.lock().unwrap().iter() {
                if let Info::Msg(msg) = info {
                    ui.label(format!("{}: {}", frame, msg));
                }
            }
        });
        app.ui.show_debug_panel = open;
        open = app.ui.find_dialog.open;
        Window::new("Find").open(&mut open).show(ctx, |ui| {
            if ui
                .text_edit_singleline(&mut app.ui.find_dialog.input)
                .lost_focus()
                && ui.input().key_pressed(egui::Key::Enter)
            {
                let needle = app.ui.find_dialog.input.parse().unwrap();
                app.ui.find_dialog.result_offsets.clear();
                for (offset, &byte) in app.data.iter().enumerate() {
                    if byte == needle {
                        app.ui.find_dialog.result_offsets.push(offset);
                    }
                }
                if let Some(&off) = app.ui.find_dialog.result_offsets.first() {
                    app.search_focus(off);
                }
            }
            ScrollArea::vertical().max_height(480.).show(ui, |ui| {
                for (i, &off) in app.ui.find_dialog.result_offsets.iter().enumerate() {
                    let re =
                        ui.selectable_label(app.ui.find_dialog.result_cursor == i, off.to_string());
                    if let Some(scroll_off) = app.ui.find_dialog.scroll_to && scroll_off == i {
                        re.scroll_to_me(None);
                        app.ui.find_dialog.scroll_to = None;
                    }
                    if re.clicked() {
                        app.search_focus(off);
                        app.ui.find_dialog.result_cursor = i;
                        break;
                    }
                }
            });
            ui.horizontal(|ui| {
                ui.set_enabled(!app.ui.find_dialog.result_offsets.is_empty());
                if (ui.button("Previous (P)").clicked() || ui.input().key_pressed(egui::Key::P))
                    && app.ui.find_dialog.result_cursor > 0
                {
                    app.ui.find_dialog.result_cursor -= 1;
                    let off = app.ui.find_dialog.result_offsets[app.ui.find_dialog.result_cursor];
                    app.search_focus(off);
                    app.ui.find_dialog.scroll_to = Some(app.ui.find_dialog.result_cursor);
                }
                ui.label((app.ui.find_dialog.result_cursor + 1).to_string());
                if (ui.button("Next (N)").clicked() || ui.input().key_pressed(egui::Key::N))
                    && app.ui.find_dialog.result_cursor + 1
                        < app.ui.find_dialog.result_offsets.len()
                {
                    app.ui.find_dialog.result_cursor += 1;
                    let off = app.ui.find_dialog.result_offsets[app.ui.find_dialog.result_cursor];
                    app.search_focus(off);
                    app.ui.find_dialog.scroll_to = Some(app.ui.find_dialog.result_cursor);
                }
                ui.label(format!(
                    "{} results",
                    app.ui.find_dialog.result_offsets.len()
                ));
            });
        });
        app.ui.find_dialog.open = open;
        TopBottomPanel::top("top_panel").show(ctx, |ui| top_panel_ui(ui, app));
        TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| bottom_panel_ui(ui, app));
        egui::SidePanel::right("right_panel").show(ctx, |ui| inspect_panel_ui(ui, app, mouse_pos));
    });
}
