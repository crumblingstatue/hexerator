use std::collections::HashSet;

use egui_sfml::egui::{self, ScrollArea, Ui};

use crate::{app::App, parse_radix::parse_guess_radix, shell::msg_warn};

#[derive(Default, Debug, PartialEq, Eq)]
pub enum FindType {
    #[default]
    U8,
    Ascii,
}

impl FindType {
    fn label(&self) -> &str {
        match self {
            FindType::U8 => "u8",
            FindType::Ascii => "ascii",
        }
    }
}

#[derive(Default, Debug)]
pub struct FindDialog {
    pub open: bool,
    pub input: String,
    /// Results, as a Bec that can be indexed. Needed because of search cursor.
    pub results_vec: Vec<usize>,
    /// Results, as a BTreeSet for fast "contains" lookup. Needed for highlighting.
    pub results_set: HashSet<usize>,
    /// Used to keep track of previous/next result to go to
    pub result_cursor: usize,
    /// When Some, the results list should be scrolled to the offset of that result
    pub scroll_to: Option<usize>,
    pub find_type: FindType,
    pub filter_results: bool,
}

impl FindDialog {
    pub fn ui(ui: &mut Ui, app: &mut App) {
        egui::ComboBox::new("type_combo", "Data type")
            .selected_text(app.ui.find_dialog.find_type.label())
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut app.ui.find_dialog.find_type,
                    FindType::U8,
                    FindType::U8.label(),
                );
                ui.selectable_value(
                    &mut app.ui.find_dialog.find_type,
                    FindType::Ascii,
                    FindType::Ascii.label(),
                );
            });
        if ui
            .text_edit_singleline(&mut app.ui.find_dialog.input)
            .lost_focus()
            && ui.input().key_pressed(egui::Key::Enter)
        {
            do_search(app);
        }
        ui.checkbox(&mut app.ui.find_dialog.filter_results, "Filter results")
            .on_hover_text("Base search on existing results");
        ui.horizontal(|ui| {
            ui.label("Offset");
            ui.separator();
            ui.label("Region");
        });
        let row_height = ui.text_style_height(&egui::TextStyle::Body);
        ScrollArea::vertical().max_height(480.).show_rows(
            ui,
            row_height,
            app.ui.find_dialog.results_vec.len(),
            |ui, range| {
                for (i, &off) in app.ui.find_dialog.results_vec[range.clone()]
                    .iter()
                    .enumerate()
                {
                    let i = i + range.start;
                    let sel_label_re = ui
                        .horizontal(|ui| {
                            let re = ui.selectable_label(
                                app.ui.find_dialog.result_cursor == i,
                                off.to_string(),
                            );
                            ui.separator();
                            match app.meta.find_most_specific_region_for_offset(off) {
                                Some(key) => {
                                    ui.label(&app.meta.regions[key].name);
                                }
                                None => {
                                    ui.label("[no region]");
                                }
                            }
                            re
                        })
                        .inner;
                    if let Some(scroll_off) = app.ui.find_dialog.scroll_to && scroll_off == i {
                        sel_label_re.scroll_to_me(None);
                        app.ui.find_dialog.scroll_to = None;
                    }
                    if sel_label_re.clicked() {
                        app.search_focus(off);
                        app.ui.find_dialog.result_cursor = i;
                        break;
                    }
                }
            },
        );
        ui.separator();
        ui.horizontal(|ui| {
            ui.set_enabled(!app.ui.find_dialog.results_vec.is_empty());
            if (ui.button("Previous (P)").clicked() || ui.input().key_pressed(egui::Key::P))
                && app.ui.find_dialog.result_cursor > 0
            {
                app.ui.find_dialog.result_cursor -= 1;
                let off = app.ui.find_dialog.results_vec[app.ui.find_dialog.result_cursor];
                app.search_focus(off);
                app.ui.find_dialog.scroll_to = Some(app.ui.find_dialog.result_cursor);
            }
            ui.label((app.ui.find_dialog.result_cursor + 1).to_string());
            if (ui.button("Next (N)").clicked() || ui.input().key_pressed(egui::Key::N))
                && app.ui.find_dialog.result_cursor + 1 < app.ui.find_dialog.results_vec.len()
            {
                app.ui.find_dialog.result_cursor += 1;
                let off = app.ui.find_dialog.results_vec[app.ui.find_dialog.result_cursor];
                app.search_focus(off);
                app.ui.find_dialog.scroll_to = Some(app.ui.find_dialog.result_cursor);
            }
            ui.label(format!("{} results", app.ui.find_dialog.results_vec.len()));
        });
    }
}

fn do_search(app: &mut App) {
    if !app.ui.find_dialog.filter_results {
        app.ui.find_dialog.results_vec.clear();
        app.ui.find_dialog.results_set.clear();
    }
    match app.ui.find_dialog.find_type {
        FindType::U8 => match parse_guess_radix(&app.ui.find_dialog.input) {
            Ok(needle) => {
                if app.ui.find_dialog.filter_results {
                    let results_vec_clone = app.ui.find_dialog.results_vec.clone();
                    app.ui.find_dialog.results_vec.clear();
                    app.ui.find_dialog.results_set.clear();
                    u8_search(
                        &mut app.ui.find_dialog,
                        results_vec_clone.iter().map(|&off| (off, app.data[off])),
                        needle,
                    );
                } else {
                    u8_search(
                        &mut app.ui.find_dialog,
                        app.data.iter().cloned().enumerate(),
                        needle,
                    );
                }
            }
            Err(e) => msg_warn(&format!("Parse fail: {}", e)),
        },
        FindType::Ascii => {
            for offset in memchr::memmem::find_iter(&app.data, &app.ui.find_dialog.input) {
                app.ui.find_dialog.results_vec.push(offset);
                app.ui.find_dialog.results_set.insert(offset);
            }
        }
    }
    if let Some(&off) = app.ui.find_dialog.results_vec.first() {
        app.search_focus(off);
    }
}

fn u8_search(dialog: &mut FindDialog, haystack: impl Iterator<Item = (usize, u8)>, needle: u8) {
    for (offset, byte) in haystack {
        if byte == needle {
            dialog.results_vec.push(offset);
            dialog.results_set.insert(offset);
        }
    }
}
