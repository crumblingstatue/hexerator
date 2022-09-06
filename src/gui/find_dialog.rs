use std::collections::HashSet;

use egui_extras::{Size, StripBuilder, TableBuilder};
use egui_sfml::egui::{self, Align, Ui};

use crate::{
    app::App,
    meta::{find_most_specific_region_for_offset, Bookmark, Meta, ValueType},
    parse_radix::parse_guess_radix,
    region_context_menu,
    shell::msg_warn,
};

use super::window_open::WindowOpen;

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

#[derive(Default)]
pub struct FindDialog {
    pub open: WindowOpen,
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
    pub fn ui(ui: &mut Ui, gui: &mut crate::gui::Gui, app: &mut App) {
        egui::ComboBox::new("type_combo", "Data type")
            .selected_text(gui.find_dialog.find_type.label())
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut gui.find_dialog.find_type,
                    FindType::U8,
                    FindType::U8.label(),
                );
                ui.selectable_value(
                    &mut gui.find_dialog.find_type,
                    FindType::Ascii,
                    FindType::Ascii.label(),
                );
            });
        let re = ui.text_edit_singleline(&mut gui.find_dialog.input);
        if gui.find_dialog.open.just_now() {
            re.request_focus();
        }
        if re.lost_focus() && ui.input().key_pressed(egui::Key::Enter) {
            do_search(app, gui);
        }
        ui.checkbox(&mut gui.find_dialog.filter_results, "Filter results")
            .on_hover_text("Base search on existing results");
        StripBuilder::new(ui).size(Size::initial(400.0)).size(Size::exact(20.0)).vertical(|mut strip| {
            strip.cell(|ui| {
                let mut action = Action::None;
                TableBuilder::new(ui)
                .striped(true)
                .columns(Size::remainder(), 4)
                .header(16.0, |mut row| {
                    row.col(|ui| {
                        ui.label("Offset");
                    });
                    row.col(|ui| {
                        ui.label("Value");
                    });
                    row.col(|ui| {
                        ui.label("Region");
                    });
                    row.col(|ui| {
                        ui.label("Bookmark");
                    });
                })
                .body(|body| {
                    body.rows(
                        20.0,
                        gui.find_dialog.results_vec.len(),
                        |i, mut row| {
                            let off = gui.find_dialog.results_vec[i];
                            let col1_re = row.col(|ui| {
                                if ui.selectable_label(
                                    gui.find_dialog.result_cursor == i,
                                    off.to_string(),
                                ).clicked() {
                                    app.search_focus(off);
                                    gui.find_dialog.result_cursor = i;
                                }
                            });
                            row.col(|ui| {
                                ui.label(app.data[off].to_string());
                            });
                            row.col(|ui| {
                                match find_most_specific_region_for_offset(&app.meta_state.meta.regions, off) {
                                    Some(key) => {
                                        let reg = &app.meta_state.meta.regions[key];
                                        if ui.link(&reg.name).context_menu(region_context_menu!(app, reg, action)).clicked() {
                                            gui.regions_window.open = true;
                                            gui.regions_window.selected_key = Some(key);
                                        }
                                    }
                                    None => {
                                        ui.label("[no region]");
                                    }
                                }
                            });
                            row.col(|ui| {
                                match Meta::bookmark_for_offset(&app.meta_state.meta.bookmarks, off) {
                                    Some((bm_idx, bm)) => {
                                        if ui.link(&bm.label).on_hover_text(&bm.desc).clicked() {
                                            gui.bookmarks_window.open.set(true);
                                            gui.bookmarks_window.selected = Some(bm_idx);
                                        }
                                    },
                                    None => { if ui.button("✚").on_hover_text("Add new bookmark").clicked() {
                                        let idx = app.meta_state.meta.bookmarks.len();
                                        app.meta_state.meta.bookmarks.push(Bookmark {
                                            offset: off,
                                            label: "New bookmark".into(),
                                            desc: String::new(),
                                            value_type: ValueType::None,
                                        });
                                        gui.bookmarks_window.open.set(true);
                                        gui.bookmarks_window.selected = Some(idx);
                                    } }
                                }
                            });
                            if let Some(scroll_off) = gui.find_dialog.scroll_to && scroll_off == i {
                                // We use center align, because it keeps the selected element in
                                // view at all times, preventing the issue of it becoming out
                                // of view, and scroll_to_me not being called because of that.
                                col1_re.scroll_to_me(Some(Align::Center));
                                gui.find_dialog.scroll_to = None;
                            }
                        },
                    );
                });
                match action {
                    Action::Goto(off) => {
                        app.center_view_on_offset(off);
                        app.edit_state.set_cursor(off);
                        app.flash_cursor();
                    }
                    Action::None => {},
                }
            });
            strip.cell(|ui| {
                ui.horizontal(|ui| {
                    ui.set_enabled(!gui.find_dialog.results_vec.is_empty());
                    if (ui.button("Previous (P)").clicked() || ui.input().key_pressed(egui::Key::P))
                        && gui.find_dialog.result_cursor > 0
                    {
                        gui.find_dialog.result_cursor -= 1;
                        let off = gui.find_dialog.results_vec[gui.find_dialog.result_cursor];
                        app.search_focus(off);
                        gui.find_dialog.scroll_to = Some(gui.find_dialog.result_cursor);
                    }
                    ui.label((gui.find_dialog.result_cursor + 1).to_string());
                    if (ui.button("Next (N)").clicked() || ui.input().key_pressed(egui::Key::N))
                        && gui.find_dialog.result_cursor + 1 < gui.find_dialog.results_vec.len()
                    {
                        gui.find_dialog.result_cursor += 1;
                        let off = gui.find_dialog.results_vec[gui.find_dialog.result_cursor];
                        app.search_focus(off);
                        gui.find_dialog.scroll_to = Some(gui.find_dialog.result_cursor);
                    }
                    ui.label(format!("{} results", gui.find_dialog.results_vec.len()));
                });
            });
        });
        gui.find_dialog.open.post_ui();
    }
}

enum Action {
    Goto(usize),
    None,
}

fn do_search(app: &mut App, gui: &mut crate::gui::Gui) {
    if !gui.find_dialog.filter_results {
        gui.find_dialog.results_vec.clear();
        gui.find_dialog.results_set.clear();
    }
    match gui.find_dialog.find_type {
        FindType::U8 => match parse_guess_radix(&gui.find_dialog.input) {
            Ok(needle) => {
                if gui.find_dialog.filter_results {
                    let results_vec_clone = gui.find_dialog.results_vec.clone();
                    gui.find_dialog.results_vec.clear();
                    gui.find_dialog.results_set.clear();
                    u8_search(
                        &mut gui.find_dialog,
                        results_vec_clone.iter().map(|&off| (off, app.data[off])),
                        needle,
                    );
                } else {
                    u8_search(
                        &mut gui.find_dialog,
                        app.data.iter().cloned().enumerate(),
                        needle,
                    );
                }
            }
            Err(e) => msg_warn(&format!("Parse fail: {}", e)),
        },
        FindType::Ascii => {
            for offset in memchr::memmem::find_iter(&app.data, &gui.find_dialog.input) {
                gui.find_dialog.results_vec.push(offset);
                gui.find_dialog.results_set.insert(offset);
            }
        }
    }
    if let Some(&off) = gui.find_dialog.results_vec.first() {
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
