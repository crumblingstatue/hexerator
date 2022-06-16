use egui_sfml::egui::{self, ScrollArea, Ui};

use crate::app::App;

#[derive(Default, Debug)]
pub struct FindDialog {
    pub open: bool,
    pub input: String,
    pub result_offsets: Vec<usize>,
    /// Used to keep track of previous/next result to go to
    pub result_cursor: usize,
    /// When Some, the results list should be scrolled to the offset of that result
    pub scroll_to: Option<usize>,
}

impl FindDialog {
    pub fn ui(ui: &mut Ui, app: &mut App) {
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
                && app.ui.find_dialog.result_cursor + 1 < app.ui.find_dialog.result_offsets.len()
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
    }
}
