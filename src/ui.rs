use std::{fmt::Debug, marker::PhantomData};

use egui_inspect::inspect;
use egui_sfml::{
    egui::{
        self, Button, ComboBox, DragValue, Layout, ScrollArea, TextEdit, TopBottomPanel, Ui, Window,
    },
    SfEgui,
};
use gamedebug_core::{per_msg, Info, PerEntry, IMMEDIATE, PERSISTENT};
use sfml::system::Vector2i;

use crate::{
    app::{App, Source},
    color::ColorMethod,
    msg_if_fail,
    slice_ext::SliceExt,
    InteractMode, Region,
};

#[expect(
    clippy::significant_drop_in_scrutinee,
    reason = "this isn't a useful lint for for loops"
)]
// https://github.com/rust-lang/rust-clippy/issues/8987
pub fn do_egui(sf_egui: &mut SfEgui, mut app: &mut App, mouse_pos: Vector2i) {
    sf_egui.do_frame(|ctx| {
        let mut open = app.show_debug_panel;
        Window::new("Debug").open(&mut open).show(ctx, |ui| {
            inspect! {
                ui,
                app
            }
            ui.separator();
            ui.heading("More Debug");
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
        app.show_debug_panel = open;
        open = app.find_dialog.open;
        Window::new("Find").open(&mut open).show(ctx, |ui| {
            if ui
                .text_edit_singleline(&mut app.find_dialog.input)
                .lost_focus()
                && ui.input().key_pressed(egui::Key::Enter)
            {
                let needle = app.find_dialog.input.parse().unwrap();
                app.find_dialog.result_offsets.clear();
                for (offset, &byte) in app.data.iter().enumerate() {
                    if byte == needle {
                        app.find_dialog.result_offsets.push(offset);
                    }
                }
                if let Some(&off) = app.find_dialog.result_offsets.first() {
                    app.search_focus(off);
                }
            }
            ScrollArea::vertical().max_height(480.).show(ui, |ui| {
                for (i, &off) in app.find_dialog.result_offsets.iter().enumerate() {
                    let re =
                        ui.selectable_label(app.find_dialog.result_cursor == i, off.to_string());
                    if let Some(scroll_off) = app.find_dialog.scroll_to && scroll_off == i {
                        re.scroll_to_me(None);
                        app.find_dialog.scroll_to = None;
                    }
                    if re.clicked() {
                        app.search_focus(off);
                        app.find_dialog.result_cursor = i;
                        break;
                    }
                }
            });
            ui.horizontal(|ui| {
                ui.set_enabled(!app.find_dialog.result_offsets.is_empty());
                if (ui.button("Previous (P)").clicked() || ui.input().key_pressed(egui::Key::P))
                    && app.find_dialog.result_cursor > 0
                {
                    app.find_dialog.result_cursor -= 1;
                    let off = app.find_dialog.result_offsets[app.find_dialog.result_cursor];
                    app.search_focus(off);
                    app.find_dialog.scroll_to = Some(app.find_dialog.result_cursor);
                }
                ui.label((app.find_dialog.result_cursor + 1).to_string());
                if (ui.button("Next (N)").clicked() || ui.input().key_pressed(egui::Key::N))
                    && app.find_dialog.result_cursor + 1 < app.find_dialog.result_offsets.len()
                {
                    app.find_dialog.result_cursor += 1;
                    let off = app.find_dialog.result_offsets[app.find_dialog.result_cursor];
                    app.search_focus(off);
                    app.find_dialog.scroll_to = Some(app.find_dialog.result_cursor);
                }
                ui.label(format!("{} results", app.find_dialog.result_offsets.len()));
            });
        });
        app.find_dialog.open = open;
        TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open").clicked() {
                        if let Some(file) = rfd::FileDialog::new().pick_file() {
                            msg_if_fail(app.load_file(file), "Failed to load file");
                        }
                        ui.close_menu();
                    }
                    if ui.button("Close").clicked() {
                        app.close_file();
                        ui.close_menu();
                    }
                });
                ui.menu_button("Seek", |ui| {
                    let re = ui
                        .button("Set cursor to initial position")
                        .on_hover_text("Set to --jump argument, 0 otherwise");
                    if re.clicked() {
                        app.set_cursor_init();
                        ui.close_menu();
                    }
                });
                ui.menu_button("View", |ui| {
                    if ui.button("Flash cursor").clicked() {
                        app.flash_cursor();
                        ui.close_menu();
                    }
                    if ui.button("Center view on cursor").clicked() {
                        app.center_view_on_offset(app.cursor);
                        app.flash_cursor();
                        ui.close_menu();
                    }
                    ui.horizontal(|ui| {
                        ui.label("Seek to byte offset");
                        let re = ui.text_edit_singleline(&mut app.seek_byte_offset_input);
                        if re.lost_focus() && ui.input().key_pressed(egui::Key::Enter) {
                            app.set_view_to_byte_offset(
                                app.seek_byte_offset_input.parse().unwrap_or(0),
                            );
                        }
                    });
                    ui.checkbox(&mut app.col_change_lock_x, "Lock x on column change");
                    ui.checkbox(&mut app.col_change_lock_y, "Lock y on column change");
                });
                ui.with_layout(Layout::right_to_left(), |ui| {
                    match &app.source {
                        Some(src) => match src {
                            Source::File(_) => {
                                match &app.args.file {
                                    Some(file) => match file.canonicalize() {
                                        Ok(path) => ui.label(path.display().to_string()),
                                        Err(e) => ui.label(format!("path error: {}", e)),
                                    },
                                    None => ui.label("File path unknown"),
                                };
                            }
                            Source::Stdin(_) => {
                                ui.label("Standard input");
                            }
                        },
                        None => {
                            ui.label("No source loaded");
                        }
                    }
                    if app.args.stream {
                        ui.label("[stream]");
                    }
                });
            });
            ui.horizontal(|ui| {
                let begin_text = match app.select_begin {
                    Some(begin) => begin.to_string(),
                    None => "-".to_owned(),
                };
                ui.label(format!("Select begin: {}", begin_text));
                if ui.button("set").clicked() {
                    match &mut app.selection {
                        Some(sel) => sel.begin = app.cursor,
                        None => app.select_begin = Some(app.cursor),
                    }
                }
                let end_text = match app.selection {
                    Some(sel) => sel.end.to_string(),
                    None => "-".to_owned(),
                };
                ui.label(format!("end: {}", end_text));
                if ui.button("set").clicked() {
                    match app.select_begin {
                        Some(begin) => match &mut app.selection {
                            None => {
                                app.selection = Some(Region {
                                    begin,
                                    end: app.cursor,
                                })
                            }
                            Some(sel) => sel.end = app.cursor,
                        },
                        None => {}
                    }
                }
                if ui.button("deselect").clicked() {
                    app.selection = None;
                }
                ui.text_edit_singleline(&mut app.fill_text);
                if ui.button("fill").clicked() {
                    if let Some(sel) = app.selection {
                        let values: Result<Vec<u8>, _> = app
                            .fill_text
                            .split(' ')
                            .map(|token| u8::from_str_radix(token, 16))
                            .collect();
                        match values {
                            Ok(values) => {
                                let range = sel.begin..=sel.end;
                                app.data[range.clone()].pattern_fill(&values);
                                app.widen_dirty_region(DamageRegion::RangeInclusive(range));
                            }
                            Err(e) => {
                                per_msg!("Fill parse error: {}", e);
                            }
                        }
                    }
                }
                ui.with_layout(Layout::right_to_left(), |ui| {
                    ui.checkbox(&mut app.invert_color, "invert");
                    ComboBox::new("color_combo", "Color")
                        .selected_text(app.color_method.name())
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut app.color_method,
                                ColorMethod::Default,
                                ColorMethod::Default.name(),
                            );
                            ui.selectable_value(
                                &mut app.color_method,
                                ColorMethod::Mono,
                                ColorMethod::Mono.name(),
                            );
                            ui.selectable_value(
                                &mut app.color_method,
                                ColorMethod::Rgb332,
                                ColorMethod::Rgb332.name(),
                            );
                            ui.selectable_value(
                                &mut app.color_method,
                                ColorMethod::Vga13h,
                                ColorMethod::Vga13h.name(),
                            );
                            ui.selectable_value(
                                &mut app.color_method,
                                ColorMethod::Grayscale,
                                ColorMethod::Grayscale.name(),
                            );
                            ui.selectable_value(
                                &mut app.color_method,
                                ColorMethod::Aitd,
                                ColorMethod::Aitd.name(),
                            );
                        });
                    ui.color_edit_button_rgb(&mut app.bg_color);
                    ui.label("Bg color");
                });
            });
        });
        TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .selectable_label(app.interact_mode == InteractMode::View, "View (F1)")
                    .clicked()
                {
                    app.interact_mode = InteractMode::View;
                }
                if ui
                    .selectable_label(app.interact_mode == InteractMode::Edit, "Edit (F2)")
                    .clicked()
                {
                    app.interact_mode = InteractMode::Edit;
                }
                ui.separator();
                match app.interact_mode {
                    InteractMode::View => {
                        ui.label("offset");
                        ui.add(DragValue::new(&mut app.view.start_offset));
                        ui.label("columns");
                        ui.add(DragValue::new(&mut app.view.cols));
                        let data_len = app.data.len();
                        if data_len != 0 {
                            let offsets = app.view_offsets();
                            ui.label(format!(
                                "view offset: row {} col {} byte {} ({:.2}%)",
                                offsets.row,
                                offsets.col,
                                offsets.byte,
                                (offsets.byte as f64 / data_len as f64) * 100.0
                            ));
                            let re = ui.add(
                                TextEdit::singleline(&mut app.center_offset_input)
                                    .hint_text("Center view on offset"),
                            );
                            if re.lost_focus() && ui.input().key_pressed(egui::Key::Enter) {
                                if let Ok(offset) = app.center_offset_input.parse() {
                                    app.center_view_on_offset(offset);
                                }
                            }
                        }
                    }
                    InteractMode::Edit => 'edit: {
                        if app.data.is_empty() {
                            break 'edit;
                        }
                        ui.label(format!("cursor: {}", app.cursor));
                        ui.separator();
                    }
                }
                ui.with_layout(Layout::right_to_left(), |ui| {
                    ui.checkbox(&mut app.show_debug_panel, "debug (F12)");
                    ui.checkbox(&mut app.show_block, "block");
                    ui.checkbox(&mut app.show_text, "text");
                    ui.checkbox(&mut app.show_hex, "hex");
                    ui.separator();
                    if ui.add(Button::new("Reload (ctrl+R)")).clicked() {
                        msg_if_fail(app.reload(), "Failed to reload");
                    }
                    if ui
                        .add_enabled(
                            !app.args.read_only && app.dirty_region.is_some(),
                            Button::new("Save (ctrl+S)"),
                        )
                        .clicked()
                    {
                        msg_if_fail(app.save(), "Failed to save");
                    }
                    ui.separator();
                    if ui.button("Restore").clicked() {
                        msg_if_fail(app.restore_backup(), "Failed to restore backup");
                    }
                    if ui.button("Backup").clicked() {
                        msg_if_fail(app.create_backup(), "Failed to create backup");
                    }
                })
            })
        });
        egui::SidePanel::right("right_panel").show(ctx, |ui| inspect_panel_ui(ui, app, mouse_pos));
    });
}

pub struct InspectPanel {
    input_thingies: [Box<dyn InputThingyTrait>; 3],
}

impl Debug for InspectPanel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InspectPanel").finish()
    }
}

impl Default for InspectPanel {
    fn default() -> Self {
        Self {
            input_thingies: [
                Box::new(InputThingy::<u8>::default()),
                Box::new(InputThingy::<u16>::default()),
                Box::new(InputThingy::<Ascii>::default()),
            ],
        }
    }
}

trait InputThingyTrait {
    fn update(&mut self, data: &[u8], offset: usize);
    fn label(&self) -> &'static str;
    fn buf_mut(&mut self) -> &mut String;
    fn write_data(&self, data: &mut [u8], offset: usize) -> Option<DamageRegion>;
}

impl<T: BytesManip> InputThingyTrait for InputThingy<T> {
    fn update(&mut self, data: &[u8], offset: usize) {
        T::update_buf(&mut self.string, data, offset);
    }
    fn label(&self) -> &'static str {
        T::label()
    }

    fn buf_mut(&mut self) -> &mut String {
        &mut self.string
    }

    fn write_data(&self, data: &mut [u8], offset: usize) -> Option<DamageRegion> {
        T::convert_and_write(&self.string, data, offset)
    }
}

pub enum DamageRegion {
    Single(usize),
    Range(std::ops::Range<usize>),
    RangeInclusive(std::ops::RangeInclusive<usize>),
}

impl BytesManip for u8 {
    fn update_buf(buf: &mut String, data: &[u8], offset: usize) {
        *buf = data[offset].to_string()
    }

    fn label() -> &'static str {
        "u8"
    }

    fn convert_and_write(buf: &str, data: &mut [u8], offset: usize) -> Option<DamageRegion> {
        match buf.parse() {
            Ok(num) => {
                data[offset] = num;
                Some(DamageRegion::Single(offset))
            }
            Err(_) => None,
        }
    }
}
impl BytesManip for u16 {
    fn update_buf(buf: &mut String, data: &[u8], offset: usize) {
        let u16 = u16::from_le_bytes(data[offset..offset + 2].try_into().unwrap());
        *buf = u16.to_string();
    }

    fn label() -> &'static str {
        "u16"
    }

    fn convert_and_write(buf: &str, data: &mut [u8], offset: usize) -> Option<DamageRegion> {
        match buf.parse::<u16>() {
            Ok(num) => {
                let range = offset..offset + 2;
                data[range.clone()].copy_from_slice(&num.to_le_bytes());
                Some(DamageRegion::Range(range))
            }
            Err(_) => None,
        }
    }
}
impl BytesManip for Ascii {
    fn update_buf(buf: &mut String, data: &[u8], offset: usize) {
        let valid_ascii_end = find_valid_ascii_end(&data[offset..]);
        *buf = String::from_utf8(data[offset..offset + valid_ascii_end].to_vec()).unwrap();
    }

    fn label() -> &'static str {
        "ascii"
    }

    fn convert_and_write(buf: &str, data: &mut [u8], offset: usize) -> Option<DamageRegion> {
        let len = buf.len();
        let range = offset..offset + len;
        data[range.clone()].copy_from_slice(buf.as_bytes());
        Some(DamageRegion::Range(range))
    }
}

struct InputThingy<T> {
    string: String,
    _phantom: PhantomData<T>,
}

impl<T> Default for InputThingy<T> {
    fn default() -> Self {
        Self {
            string: Default::default(),
            _phantom: Default::default(),
        }
    }
}

trait BytesManip {
    fn update_buf(buf: &mut String, data: &[u8], offset: usize);
    fn label() -> &'static str;
    fn convert_and_write(buf: &str, data: &mut [u8], offset: usize) -> Option<DamageRegion>;
}

struct Ascii;

fn inspect_panel_ui(ui: &mut Ui, app: &mut App, mouse_pos: Vector2i) {
    let offset = match app.interact_mode {
        InteractMode::View => {
            let off = app.pixel_pos_byte_offset(mouse_pos.x, mouse_pos.y);
            ui.label(format!("Pointer at {} (0x{:x})", off, off));
            off
        }
        InteractMode::Edit => {
            ui.label(format!("Cursor at {} ({:x}h)", app.cursor, app.cursor));
            app.cursor
        }
    };
    if app.data.is_empty() {
        return;
    }
    if offset != app.prev_frame_inspect_offset {
        for thingy in &mut app.inspect_panel.input_thingies {
            thingy.update(&app.data[..], offset);
        }
    }
    let mut damages = Vec::new();
    for thingy in &mut app.inspect_panel.input_thingies {
        ui.label(thingy.label());
        if ui.text_edit_singleline(thingy.buf_mut()).lost_focus()
            && ui.input().key_pressed(egui::Key::Enter)
        {
            if let Some(range) = thingy.write_data(&mut app.data, offset) {
                damages.push(range);
            }
        }
    }
    for damage in damages {
        app.widen_dirty_region(damage);
    }
    app.prev_frame_inspect_offset = offset;
}

fn find_valid_ascii_end(data: &[u8]) -> usize {
    data.iter()
        .position(|&b| b == 0 || b > 127)
        .unwrap_or(data.len())
}
impl DamageRegion {
    pub(crate) fn begin(&self) -> usize {
        match self {
            DamageRegion::Single(offset) => *offset,
            DamageRegion::Range(range) => range.start,
            DamageRegion::RangeInclusive(range) => *range.start(),
        }
    }

    pub(crate) fn end(&self) -> usize {
        match self {
            DamageRegion::Single(offset) => *offset,
            DamageRegion::Range(range) => range.end - 1,
            DamageRegion::RangeInclusive(range) => *range.end(),
        }
    }
}
