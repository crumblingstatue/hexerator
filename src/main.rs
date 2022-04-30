#![feature(let_chains)]

mod hex_conv;

use egui_inspect::{derive::Inspect, inspect};
use egui_sfml::{
    egui::{self, color::rgb_from_hsv, Button, Layout, TextEdit, TopBottomPanel, Window},
    SfEgui,
};
use gamedebug_core::{Info, PerEntry, PERSISTENT};
use sfml::{
    graphics::{
        Color, Font, PrimitiveType, Rect, RectangleShape, RenderStates, RenderTarget, RenderWindow,
        Shape, Vertex,
    },
    system::Vector2,
    window::{mouse, ContextSettings, Event, Key, Style},
};

#[derive(PartialEq, Debug, Inspect)]
enum EditTarget {
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
#[derive(PartialEq, Debug, Inspect)]
enum InteractMode {
    /// Mode optimized for viewing the contents
    ///
    /// For example arrow keys scroll the content
    View,
    /// Mode optimized for editing the contents
    ///
    /// For example arrow keys move the cursor
    Edit,
}

fn main() {
    let path = std::env::args_os()
        .nth(1)
        .expect("Need file path as argument");
    let mut data = std::fs::read(&path).unwrap();
    let mut w = RenderWindow::new(
        (1920, 1080),
        "hello",
        Style::NONE,
        &ContextSettings::default(),
    );
    w.set_vertical_sync_enabled(true);
    w.set_position(Vector2::new(0, 0));
    let mut sf_egui = SfEgui::new(&w);
    let f = Font::from_memory(include_bytes!("../DejaVuSansMono.ttf")).unwrap();
    let mut vertices = Vec::new();
    let mut rows = 66;
    // Number of columns in the view
    let mut cols = 48;
    // Maximum number of visible cols that can be shown on screen
    let mut max_visible_cols = 74;
    let mut starting_offset: usize = 0;
    let mut colorize = true;
    // The editing byte offset
    let mut cursor: usize = 0;
    // The value of the cursor on the previous frame. Used to determine when the cursor changes
    let mut cursor_prev_frame = cursor;
    let mut edit_target = EditTarget::Hex;
    let mut dirty = false;
    let mut row_height: u8 = 16;
    let mut col_width: u8 = 26;
    let mut show_text = true;
    let mut interact_mode = InteractMode::View;
    // The half digit when the user begins to type into a hex view
    let mut hex_edit_half_digit = None;
    let mut show_debug_panel = false;
    let mut u8_buf = String::new();

    while w.is_open() {
        // region: event handling
        while let Some(event) = w.poll_event() {
            sf_egui.add_event(&event);
            let wants_pointer = sf_egui.context().wants_pointer_input();
            let wants_kb = sf_egui.context().wants_keyboard_input();
            if wants_kb {
                if event == Event::Closed {
                    w.close();
                }
                continue;
            }
            match event {
                Event::Closed => w.close(),
                Event::KeyPressed {
                    code, shift, ctrl, ..
                } => match code {
                    Key::Up => match interact_mode {
                        InteractMode::View => {
                            starting_offset = starting_offset.saturating_sub(cols)
                        }
                        InteractMode::Edit => {
                            cursor = cursor.saturating_sub(cols);
                            if cursor < starting_offset {
                                starting_offset -= cols;
                            }
                        }
                    },
                    Key::Down => match interact_mode {
                        InteractMode::View => starting_offset += cols,
                        InteractMode::Edit => {
                            if cursor + cols < data.len() {
                                cursor += cols;
                            }
                            if cursor >= starting_offset + rows * cols {
                                starting_offset += cols;
                            }
                        }
                    },
                    Key::Left => match interact_mode {
                        InteractMode::View => {
                            if ctrl {
                                cols = cols.saturating_sub(1);
                            } else {
                                starting_offset = starting_offset.saturating_sub(1);
                            }
                        }
                        InteractMode::Edit => cursor = cursor.saturating_sub(1),
                    },
                    Key::Right => match interact_mode {
                        InteractMode::View => {
                            if ctrl {
                                cols += 1;
                            } else {
                                starting_offset += 1;
                            }
                        }
                        InteractMode::Edit => {
                            if cursor + 1 < data.len() {
                                cursor += 1;
                            }
                        }
                    },
                    Key::PageUp => {
                        let amount = rows * cols;
                        if starting_offset >= amount {
                            starting_offset -= amount;
                            if interact_mode == InteractMode::Edit {
                                cursor = cursor.saturating_sub(amount);
                            }
                        } else {
                            starting_offset = 0
                        }
                    }
                    Key::PageDown => {
                        let amount = rows * cols;
                        if starting_offset + amount < data.len() {
                            starting_offset += amount;
                            if interact_mode == InteractMode::Edit && cursor + amount < data.len() {
                                cursor += amount;
                            }
                        }
                    }
                    Key::Home => {
                        starting_offset = 0;
                        if interact_mode == InteractMode::Edit {
                            cursor = 0;
                        }
                    }
                    Key::End => {
                        let pos = data.len() - rows * cols;
                        starting_offset = pos;
                        if interact_mode == InteractMode::Edit {
                            cursor = pos;
                        }
                    }
                    Key::Tab if shift => {
                        edit_target.switch();
                        hex_edit_half_digit = None;
                    }
                    Key::F1 => interact_mode = InteractMode::View,
                    Key::F2 => interact_mode = InteractMode::Edit,
                    Key::F12 => show_debug_panel ^= true,
                    Key::Escape => {
                        hex_edit_half_digit = None;
                    }
                    _ => {}
                },
                Event::TextEntered { unicode } => match interact_mode {
                    InteractMode::Edit => match edit_target {
                        EditTarget::Hex => {
                            if unicode.is_ascii() {
                                let ascii = unicode as u8;
                                if (b'0'..=b'f').contains(&ascii) {
                                    match hex_edit_half_digit {
                                        Some(half) => {
                                            data[cursor] = hex_conv::merge_hex_halves(half, ascii);
                                            dirty = true;
                                            if cursor + 1 < data.len() {
                                                cursor += 1;
                                            }
                                            hex_edit_half_digit = None;
                                        }
                                        None => hex_edit_half_digit = Some(ascii),
                                    }
                                }
                            }
                        }
                        EditTarget::Text => {
                            if unicode.is_ascii() {
                                data[cursor] = unicode as u8;
                                dirty = true;
                                if cursor + 1 < data.len() {
                                    cursor += 1;
                                }
                            }
                        }
                    },
                    InteractMode::View => {}
                },
                Event::MouseButtonPressed { button, x, y } if !wants_pointer => {
                    if button == mouse::Button::Left {
                        let row = y as usize / usize::from(row_height);
                        let col = x as usize / usize::from(col_width);
                        let new_cursor = row * cols + col;
                        cursor = starting_offset + new_cursor;
                    }
                }
                _ => {}
            }
        }
        let cursor_changed = cursor != cursor_prev_frame;
        if cursor_changed {
            u8_buf = data[cursor].to_string();
        }
        // endregion
        w.clear(Color::BLACK);
        let mut rs = RenderStates::default();
        vertices.clear();
        sf_egui.do_frame(|ctx| {
            Window::new("Debug")
                .open(&mut show_debug_panel)
                .show(ctx, |ui| {
                    // region: debug panel
                    inspect! {
                        ui,
                        rows,
                        cols,
                        max_visible_cols,
                        starting_offset,
                        cursor,
                        edit_target,
                        row_height,
                        col_width
                    }
                    ui.separator();
                    if ui.add_enabled(dirty, Button::new("Reload")).clicked() {
                        data = std::fs::read(&path).unwrap();
                        dirty = false;
                    }
                    if ui.add_enabled(dirty, Button::new("Save")).clicked() {
                        std::fs::write(&path, &data).unwrap();
                    }
                    ui.separator();
                    ui.heading("Debug log");
                    for PerEntry { frame, info } in PERSISTENT.lock().unwrap().iter() {
                        if let Info::Msg(msg) = info {
                            ui.label(format!("{}: {}", frame, msg));
                        }
                    }
                    // endregion
                });
            // region: bottom panel
            TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .selectable_label(interact_mode == InteractMode::View, "View (F1)")
                        .clicked()
                    {
                        interact_mode = InteractMode::View;
                    }
                    if ui
                        .selectable_label(interact_mode == InteractMode::Edit, "Edit (F2)")
                        .clicked()
                    {
                        interact_mode = InteractMode::Edit;
                    }
                    ui.separator();
                    match interact_mode {
                        InteractMode::View => {
                            ui.label(format!("offset: {}", starting_offset));
                            ui.label(format!("columns: {}", cols));
                        }
                        InteractMode::Edit => {
                            ui.label(format!("cursor: {}", cursor));
                            ui.separator();
                            ui.label("u8");
                            if ui
                                .add(TextEdit::singleline(&mut u8_buf).desired_width(28.0))
                                .lost_focus()
                                && ui.input().key_pressed(egui::Key::Enter)
                            {
                                data[cursor] = u8_buf.parse().unwrap();
                                dirty = true;
                            }
                        }
                    }
                    ui.with_layout(Layout::right_to_left(), |ui| {
                        ui.checkbox(&mut show_debug_panel, "debug (F12)");
                        ui.checkbox(&mut colorize, "color");
                        ui.checkbox(&mut show_text, "text");
                    })
                })
            });
            // endregion
        });
        // region: hex display
        let mut idx = starting_offset;
        'display: for y in 0..rows {
            for x in 0..cols {
                if x == max_visible_cols {
                    idx += cols - x;
                    break;
                }
                if idx >= data.len() {
                    break 'display;
                }
                let byte = data[idx];
                if idx == cursor {
                    let extra_x = if hex_edit_half_digit.is_none() {
                        0
                    } else {
                        col_width / 2
                    };
                    draw_cursor(
                        x as f32 * f32::from(col_width) + extra_x as f32,
                        y as f32 * f32::from(row_height),
                        &mut w,
                        edit_target == EditTarget::Hex && interact_mode == InteractMode::Edit,
                    );
                }
                let [mut g1, g2] = hex_conv::byte_to_hex_digits(byte);
                if let Some(half) = hex_edit_half_digit && cursor == idx {
                    g1 = half.to_ascii_uppercase();
                }
                let [r, g, b] = rgb_from_hsv((byte as f32 / 255.0, 1.0, 1.0));
                let c = if colorize {
                    Color::rgb((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
                } else {
                    Color::WHITE
                };
                draw_glyph(
                    &f,
                    &mut vertices,
                    x as f32 * f32::from(col_width),
                    y as f32 * f32::from(row_height),
                    g1 as u32,
                    c,
                );
                draw_glyph(
                    &f,
                    &mut vertices,
                    x as f32 * f32::from(col_width) + 11.0,
                    y as f32 * f32::from(row_height),
                    g2 as u32,
                    c,
                );
                idx += 1;
            }
        }
        // endregion
        // region: ascii display
        if show_text {
            idx = starting_offset;
            'asciidisplay: for y in 0..rows {
                for x in 0..cols {
                    if x == max_visible_cols {
                        idx += cols - x;
                        break;
                    }
                    if idx >= data.len() {
                        break 'asciidisplay;
                    }
                    let byte = data[idx];
                    let [r, g, b] = rgb_from_hsv((byte as f32 / 255.0, 1.0, 1.0));
                    let c = if colorize {
                        Color::rgb((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
                    } else {
                        Color::WHITE
                    };
                    if idx == cursor {
                        draw_cursor(
                            (x + cols * 2 + 1) as f32 * f32::from(col_width / 2),
                            y as f32 * f32::from(row_height),
                            &mut w,
                            edit_target == EditTarget::Text && interact_mode == InteractMode::Edit,
                        );
                    }
                    draw_glyph(
                        &f,
                        &mut vertices,
                        (x + cols * 2 + 1) as f32 * f32::from(col_width / 2),
                        y as f32 * f32::from(row_height),
                        byte as u32,
                        c,
                    );
                    idx += 1;
                }
            }
        }
        // endregion
        rs.set_texture(Some(f.texture(10)));
        w.draw_primitives(&vertices, PrimitiveType::QUADS, &rs);
        rs.set_texture(None);
        sf_egui.draw(&mut w, None);
        w.display();
        gamedebug_core::inc_frame();
        cursor_prev_frame = cursor;
    }
}

fn draw_cursor(x: f32, y: f32, w: &mut RenderWindow, active: bool) {
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
    w.draw(&rs);
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
