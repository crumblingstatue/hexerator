#![feature(let_chains)]

mod app;
mod hex_conv;
mod input;
mod slice_ext;

use crate::{app::App, slice_ext::SliceExt};
use egui_inspect::{derive::Inspect, inspect};
use egui_sfml::{
    egui::{
        self, color::rgb_from_hsv, Button, Layout, ScrollArea, TextEdit, TopBottomPanel, Window,
    },
    SfEgui,
};
use gamedebug_core::{imm_msg, per_msg, Info, PerEntry, IMMEDIATE, PERSISTENT};
use sfml::{
    graphics::{
        Color, Font, PrimitiveType, Rect, RectangleShape, RenderStates, RenderTarget, RenderWindow,
        Shape, Vertex,
    },
    system::Vector2,
    window::{mouse, ContextSettings, Event, Key, Style},
};

#[derive(PartialEq, Eq, Debug, Inspect)]
pub enum EditTarget {
    Hex,
    Text,
}

impl EditTarget {
    fn switch(&mut self) {
        *self = match self {
            EditTarget::Hex => EditTarget::Text,
            EditTarget::Text => EditTarget::Hex,
        }
    }
}

/// User interaction mode
///
/// There are 2 modes: View and Edit
#[derive(PartialEq, Eq, Debug, Inspect)]
pub enum InteractMode {
    /// Mode optimized for viewing the contents
    ///
    /// For example arrow keys scroll the content
    View,
    /// Mode optimized for editing the contents
    ///
    /// For example arrow keys move the cursor
    Edit,
}

#[derive(Default, Debug, Inspect)]
pub struct FindDialog {
    open: bool,
    input: String,
    result_offsets: Vec<usize>,
    /// Used to keep track of previous/next result to go to
    result_cursor: usize,
    /// When Some, the results list should be scrolled to the offset of that result
    scroll_to: Option<usize>,
}

#[derive(Clone, Copy, Debug, Inspect)]
pub struct Region {
    begin: usize,
    end: usize,
}

fn main() {
    let path = std::env::args_os()
        .nth(1)
        .expect("Need file path as argument");
    let mut window = RenderWindow::new(
        (1920, 1080),
        "Hexerator",
        Style::NONE,
        &ContextSettings::default(),
    );
    window.set_vertical_sync_enabled(true);
    window.set_position(Vector2::new(0, 0));
    let mut sf_egui = SfEgui::new(&window);
    let font = unsafe { Font::from_memory(include_bytes!("../DejaVuSansMono.ttf")).unwrap() };
    let mut app = App::new(path);

    while window.is_open() {
        do_frame(&mut app, &mut sf_egui, &mut window, &font);
    }
}

fn do_frame(app: &mut App, sf_egui: &mut SfEgui, window: &mut RenderWindow, font: &Font) {
    handle_events(app, window, sf_egui);
    update(app);
    do_egui(sf_egui, app);
    window.clear(Color::BLACK);
    draw(app, window, font);
    sf_egui.draw(window, None);
    window.display();
    gamedebug_core::inc_frame();
    app.cursor_prev_frame = app.cursor;
}

fn update(app: &mut App) {
    if app.interact_mode == InteractMode::View && !app.input.key_down(Key::LControl) {
        let spd = if app.input.key_down(Key::LShift) {
            app.scroll_speed * 4
        } else {
            app.scroll_speed
        };
        if app.input.key_down(Key::Left) {
            app.view_x -= spd;
        } else if app.input.key_down(Key::Right) {
            app.view_x += spd;
        }
        if app.input.key_down(Key::Up) {
            app.view_y -= spd;
        } else if app.input.key_down(Key::Down) {
            app.view_y += spd;
        }
    }
    let cursor_changed = app.cursor != app.cursor_prev_frame;
    if cursor_changed {
        app.u8_buf = app.data[app.cursor].to_string();
    }
}

fn draw(app: &mut App, window: &mut RenderWindow, font: &Font) {
    let mut rs = RenderStates::default();
    app.vertices.clear();
    // The offset for the hex display imposed by the view
    let view_idx_off_x: usize = app.view_x.try_into().unwrap_or(0) / app.col_width as usize;
    let view_idx_off_y: usize = app.view_y.try_into().unwrap_or(0) / app.row_height as usize;
    let view_idx_off = view_idx_off_y * app.view.cols + view_idx_off_x;
    // The ascii view has a different offset indexing
    imm_msg!(view_idx_off_x);
    imm_msg!(view_idx_off_y);
    imm_msg!(view_idx_off);
    let mut idx = app.view.start_offset + view_idx_off;
    let mut rows_rendered: u32 = 0;
    let mut cols_rendered: u32 = 0;
    'display: for y in 0..app.view.rows {
        for x in 0..app.view.cols {
            if x == app.max_visible_cols || x >= app.view.cols.saturating_sub(view_idx_off_x) {
                idx += app.view.cols - x;
                break;
            }
            if idx >= app.data.len() {
                break 'display;
            }
            let pix_x = (x + view_idx_off_x) as f32 * f32::from(app.col_width) - app.view_x as f32;
            let pix_y = (y + view_idx_off_y) as f32 * f32::from(app.row_height) - app.view_y as f32;
            let byte = app.data[idx];
            let selected = match app.selection {
                Some(sel) => (sel.begin..=sel.end).contains(&idx),
                None => false,
            };
            if selected || (app.find_dialog.open && app.find_dialog.result_offsets.contains(&idx)) {
                let mut rs = RectangleShape::from_rect(Rect::new(
                    pix_x,
                    pix_y,
                    app.col_width as f32,
                    app.row_height as f32,
                ));
                rs.set_fill_color(Color::rgb(150, 150, 150));
                if app.cursor == idx {
                    rs.set_outline_color(Color::WHITE);
                    rs.set_outline_thickness(-2.0);
                }
                window.draw(&rs);
            }
            if idx == app.cursor {
                let extra_x = if app.hex_edit_half_digit.is_none() {
                    0
                } else {
                    app.col_width / 2
                };
                draw_cursor(
                    pix_x + extra_x as f32,
                    pix_y,
                    window,
                    app.edit_target == EditTarget::Hex && app.interact_mode == InteractMode::Edit,
                );
            }
            let [mut g1, g2] = hex_conv::byte_to_hex_digits(byte);
            if let Some(half) = app.hex_edit_half_digit && app.cursor == idx {
                g1 = half.to_ascii_uppercase();
            }
            let c = byte_color(byte, !app.colorize);
            draw_glyph(font, &mut app.vertices, pix_x, pix_y, g1 as u32, c);
            draw_glyph(font, &mut app.vertices, pix_x + 11.0, pix_y, g2 as u32, c);
            idx += 1;
            cols_rendered += 1;
        }
        rows_rendered += 1;
    }
    imm_msg!(rows_rendered);
    cols_rendered = cols_rendered.checked_div(rows_rendered).unwrap_or(0);
    imm_msg!(cols_rendered);
    // The offset for the ascii display imposed by the view
    let ascii_display_x_offset = app.ascii_display_x_offset();
    imm_msg!(ascii_display_x_offset);
    let view_idx_off_x: usize = app
        .view_x
        .saturating_sub(ascii_display_x_offset)
        .try_into()
        .unwrap_or(0)
        / app.col_width as usize;
    //let view_idx_off_y: usize = app.view_y.try_into().unwrap_or(0) / app.row_height as usize;
    let view_idx_off = view_idx_off_y * app.view.cols + view_idx_off_x;
    imm_msg!("ascii");
    imm_msg!(view_idx_off_x);
    //imm_msg!(view_idx_off_y);
    imm_msg!(view_idx_off);
    let mut ascii_rows_rendered: u32 = 0;
    let mut ascii_cols_rendered: u32 = 0;
    if app.show_text {
        idx = app.view.start_offset + view_idx_off;
        imm_msg!(idx);
        'asciidisplay: for y in 0..app.view.rows {
            for x in 0..app.view.cols {
                if x == app.max_visible_cols * 2
                    || x >= app.view.cols.saturating_sub(view_idx_off_x)
                {
                    idx += app.view.cols - x;
                    break;
                }
                if idx >= app.data.len() {
                    break 'asciidisplay;
                }
                let pix_x = (x + app.view.cols * 2 + 1) as f32 * f32::from(app.col_width / 2)
                    - app.view_x as f32;
                //let pix_y = y as f32 * f32::from(app.row_height) - app.view_y as f32;
                let pix_y =
                    (y + view_idx_off_y) as f32 * f32::from(app.row_height) - app.view_y as f32;
                let byte = app.data[idx];
                let c = byte_color(byte, !app.colorize);
                let selected = match app.selection {
                    Some(sel) => (sel.begin..=sel.end).contains(&idx),
                    None => false,
                };
                if selected
                    || (app.find_dialog.open && app.find_dialog.result_offsets.contains(&idx))
                {
                    let mut rs = RectangleShape::from_rect(Rect::new(
                        pix_x,
                        pix_y,
                        (app.col_width / 2) as f32,
                        app.row_height as f32,
                    ));
                    rs.set_fill_color(Color::rgb(150, 150, 150));
                    if app.cursor == idx {
                        rs.set_outline_color(Color::WHITE);
                        rs.set_outline_thickness(-2.0);
                    }
                    window.draw(&rs);
                }
                if idx == app.cursor {
                    draw_cursor(
                        pix_x,
                        pix_y,
                        window,
                        app.edit_target == EditTarget::Text
                            && app.interact_mode == InteractMode::Edit,
                    );
                }
                draw_glyph(font, &mut app.vertices, pix_x, pix_y, byte as u32, c);
                idx += 1;
                ascii_cols_rendered += 1;
            }
            ascii_rows_rendered += 1;
        }
    }
    imm_msg!(ascii_rows_rendered);
    ascii_cols_rendered = ascii_cols_rendered
        .checked_div(ascii_rows_rendered)
        .unwrap_or(0);
    imm_msg!(ascii_cols_rendered);
    rs.set_texture(Some(font.texture(10)));
    window.draw_primitives(&app.vertices, PrimitiveType::QUADS, &rs);
    rs.set_texture(None);
}

fn handle_events(app: &mut App, window: &mut RenderWindow, sf_egui: &mut SfEgui) {
    while let Some(event) = window.poll_event() {
        app.input.update_from_event(&event);
        sf_egui.add_event(&event);
        let wants_pointer = sf_egui.context().wants_pointer_input();
        let wants_kb = sf_egui.context().wants_keyboard_input();
        if wants_kb {
            if event == Event::Closed {
                window.close();
            }
            continue;
        }
        match event {
            Event::Closed => window.close(),
            Event::KeyPressed {
                code, shift, ctrl, ..
            } => match code {
                Key::Up => match app.interact_mode {
                    InteractMode::View => {
                        if ctrl {
                            app.view.start_offset = app.view.start_offset.saturating_sub(1);
                        }
                    }
                    InteractMode::Edit => {
                        app.cursor = app.cursor.saturating_sub(app.view.cols);
                    }
                },
                Key::Down => match app.interact_mode {
                    InteractMode::View => {
                        if ctrl {
                            app.view.start_offset += 1;
                        }
                    }
                    InteractMode::Edit => {
                        if app.cursor + app.view.cols < app.data.len() {
                            app.cursor += app.view.cols;
                        }
                    }
                },
                Key::Left => {
                    if app.interact_mode == InteractMode::Edit {
                        app.cursor = app.cursor.saturating_sub(1)
                    } else if ctrl {
                        app.view.cols -= 1;
                    }
                }
                Key::Right => {
                    if app.interact_mode == InteractMode::Edit && app.cursor + 1 < app.data.len() {
                        app.cursor += 1;
                    } else if ctrl {
                        app.view.cols += 1;
                    }
                }
                Key::PageUp => match app.interact_mode {
                    InteractMode::View => {
                        app.view_y -= 1040;
                    }
                    InteractMode::Edit => {
                        let amount = app.view.rows * app.view.cols;
                        if app.view.start_offset >= amount {
                            app.view.start_offset -= amount;
                            if app.interact_mode == InteractMode::Edit {
                                app.cursor = app.cursor.saturating_sub(amount);
                            }
                        } else {
                            app.view.start_offset = 0
                        }
                    }
                },
                Key::PageDown => match app.interact_mode {
                    InteractMode::View => app.view_y += 1040,
                    InteractMode::Edit => {
                        let amount = app.view.rows * app.view.cols;
                        if app.view.start_offset + amount < app.data.len() {
                            app.view.start_offset += amount;
                            if app.interact_mode == InteractMode::Edit
                                && app.cursor + amount < app.data.len()
                            {
                                app.cursor += amount;
                            }
                        }
                    }
                },
                Key::Home => match app.interact_mode {
                    InteractMode::View => app.view_y = -app.top_gap,
                    InteractMode::Edit => {
                        app.view.start_offset = 0;
                        app.cursor = 0;
                    }
                },
                Key::End => match app.interact_mode {
                    InteractMode::View => {
                        let data_pix_size =
                            (app.data.len() / app.view.cols) as i64 * i64::from(app.row_height);
                        app.view_y = data_pix_size - 1040;
                    }
                    InteractMode::Edit => {
                        let pos = app.data.len() - app.view.rows * app.view.cols;
                        app.view.start_offset = pos;
                        if app.interact_mode == InteractMode::Edit {
                            app.cursor = pos;
                        }
                    }
                },
                Key::Tab if shift => {
                    app.edit_target.switch();
                    app.hex_edit_half_digit = None;
                }
                Key::F1 => app.interact_mode = InteractMode::View,
                Key::F2 => app.interact_mode = InteractMode::Edit,
                Key::F12 => app.toggle_debug(),
                Key::Escape => {
                    app.hex_edit_half_digit = None;
                }
                Key::F if ctrl => {
                    app.find_dialog.open ^= true;
                }
                Key::S if ctrl => {
                    app.save();
                }
                Key::R if ctrl => {
                    app.reload();
                }
                _ => {}
            },
            Event::TextEntered { unicode } => match app.interact_mode {
                InteractMode::Edit => match app.edit_target {
                    EditTarget::Hex => {
                        if unicode.is_ascii() {
                            let ascii = unicode as u8;
                            if (b'0'..=b'f').contains(&ascii) {
                                match app.hex_edit_half_digit {
                                    Some(half) => {
                                        app.data[app.cursor] =
                                            hex_conv::merge_hex_halves(half, ascii);
                                        app.dirty = true;
                                        if app.cursor + 1 < app.data.len() {
                                            app.cursor += 1;
                                        }
                                        app.hex_edit_half_digit = None;
                                    }
                                    None => app.hex_edit_half_digit = Some(ascii),
                                }
                            }
                        }
                    }
                    EditTarget::Text => {
                        if unicode.is_ascii() {
                            app.data[app.cursor] = unicode as u8;
                            app.dirty = true;
                            if app.cursor + 1 < app.data.len() {
                                app.cursor += 1;
                            }
                        }
                    }
                },
                InteractMode::View => {}
            },
            Event::MouseButtonPressed { button, x, y } if !wants_pointer => {
                if button == mouse::Button::Left {
                    let x: i64 = app.view_x + i64::from(x);
                    let y: i64 = app.view_y + i64::from(y);
                    per_msg!("x: {}, y: {}", x, y);
                    let ascii_display_x_offset = app.ascii_display_x_offset();
                    let col_x;
                    let col_y = y / i64::from(app.row_height);
                    if x < ascii_display_x_offset {
                        col_x = x / i64::from(app.col_width);
                        per_msg!("col_x: {}, col_y: {}", col_x, col_y);
                    } else {
                        let x_rel = x - ascii_display_x_offset;
                        col_x = x_rel / i64::from(app.col_width / 2);
                    }
                    let new_cursor = usize::try_from(col_y).unwrap_or(0) * app.view.cols
                        + usize::try_from(col_x).unwrap_or(0);
                    app.cursor = app.view.start_offset + new_cursor;
                }
            }
            _ => {}
        }
    }
}

fn do_egui(sf_egui: &mut SfEgui, mut app: &mut App) {
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
        Window::new("Find")
            .open(&mut app.find_dialog.open)
            .show(ctx, |ui| {
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
                        App::search_focus(&mut app.cursor, &mut app.view, off);
                    }
                }
                ScrollArea::vertical().max_height(480.).show(ui, |ui| {
                    for (i, &off) in app.find_dialog.result_offsets.iter().enumerate() {
                        let re = ui
                            .selectable_label(app.find_dialog.result_cursor == i, off.to_string());
                        if let Some(scroll_off) = app.find_dialog.scroll_to && scroll_off == i {
                        re.scroll_to_me(None);
                        app.find_dialog.scroll_to = None;
                    }
                        if re.clicked() {
                            App::search_focus(&mut app.cursor, &mut app.view, off);
                            app.find_dialog.result_cursor = i;
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
                        App::search_focus(&mut app.cursor, &mut app.view, off);
                        app.find_dialog.scroll_to = Some(app.find_dialog.result_cursor);
                    }
                    ui.label((app.find_dialog.result_cursor + 1).to_string());
                    if (ui.button("Next (N)").clicked() || ui.input().key_pressed(egui::Key::N))
                        && app.find_dialog.result_cursor + 1 < app.find_dialog.result_offsets.len()
                    {
                        app.find_dialog.result_cursor += 1;
                        let off = app.find_dialog.result_offsets[app.find_dialog.result_cursor];
                        App::search_focus(&mut app.cursor, &mut app.view, off);
                        app.find_dialog.scroll_to = Some(app.find_dialog.result_cursor);
                    }
                    ui.label(format!("{} results", app.find_dialog.result_offsets.len()));
                });
            });
        TopBottomPanel::top("top_panel").show(ctx, |ui| {
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
                                app.data[sel.begin..=sel.end].pattern_fill(&values);
                                app.dirty = true;
                            }
                            Err(e) => {
                                per_msg!("Fill parse error: {}", e);
                            }
                        }
                    }
                }
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
                        ui.label(format!("offset: {}", app.view.start_offset));
                        ui.label(format!("columns: {}", app.view.cols));
                    }
                    InteractMode::Edit => {
                        ui.label(format!("app.cursor: {}", app.cursor));
                        ui.separator();
                        ui.label("u8");
                        if ui
                            .add(TextEdit::singleline(&mut app.u8_buf).desired_width(28.0))
                            .lost_focus()
                            && ui.input().key_pressed(egui::Key::Enter)
                        {
                            app.data[app.cursor] = app.u8_buf.parse().unwrap();
                            app.dirty = true;
                        }
                        ui.label("ascii");
                        ui.add(
                            TextEdit::singleline(&mut (app.data[app.cursor] as char).to_string())
                                .desired_width(28.0),
                        );
                    }
                }
                ui.with_layout(Layout::right_to_left(), |ui| {
                    ui.checkbox(&mut app.show_debug_panel, "debug (F12)");
                    ui.checkbox(&mut app.colorize, "color");
                    ui.checkbox(&mut app.show_text, "text");
                    ui.separator();
                    if ui
                        .add_enabled(app.dirty, Button::new("Reload (ctrl+R)"))
                        .clicked()
                    {
                        app.reload();
                    }
                    if ui
                        .add_enabled(app.dirty, Button::new("Save (ctrl+S)"))
                        .clicked()
                    {
                        app.save();
                    }
                    ui.separator();
                    if ui.button("Restore").clicked() {
                        std::fs::copy(&app.backup_path, &app.path).unwrap();
                        app.reload();
                    }
                    if ui.button("Backup").clicked() {
                        std::fs::copy(&app.path, &app.backup_path).unwrap();
                    }
                })
            })
        });
    });
}

fn byte_color(byte: u8, mono: bool) -> Color {
    if mono {
        Color::WHITE
    } else if byte == 0 {
        Color::rgb(100, 100, 100)
    } else if byte == 255 {
        Color::WHITE
    } else {
        let [r, g, b] = rgb_from_hsv((byte as f32 / 288.0, 1.0, 1.0));
        Color::rgb((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
    }
}

fn draw_cursor(x: f32, y: f32, window: &mut RenderWindow, active: bool) {
    let mut rs = RectangleShape::from_rect(Rect {
        left: x,
        top: y,
        width: 10.0,
        height: 10.0,
    });
    rs.set_fill_color(Color::TRANSPARENT);
    rs.set_outline_thickness(2.0);
    if active {
        rs.set_outline_color(Color::WHITE);
    } else {
        rs.set_outline_color(Color::rgb(150, 150, 150));
    }
    window.draw(&rs);
}

fn draw_glyph(font: &Font, vertices: &mut Vec<Vertex>, x: f32, y: f32, glyph: u32, color: Color) {
    let g = font.glyph(glyph, 10, false, 0.0);
    let r = g.texture_rect();
    vertices.push(Vertex {
        position: Vector2::new(x, y),
        color,
        tex_coords: Vector2::new(r.left as f32, r.top as f32),
    });
    vertices.push(Vertex {
        position: Vector2::new(x, y + 10.0),
        color,
        tex_coords: Vector2::new(r.left as f32, (r.top + r.height) as f32),
    });
    vertices.push(Vertex {
        position: Vector2::new(x + 10.0, y + 10.0),
        color,
        tex_coords: Vector2::new((r.left + r.width) as f32, (r.top + r.height) as f32),
    });
    vertices.push(Vertex {
        position: Vector2::new(x + 10.0, y),
        color,
        tex_coords: Vector2::new((r.left + r.width) as f32, r.top as f32),
    });
}
