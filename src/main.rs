#![feature(let_chains, lint_reasons)]

mod app;
mod args;
mod color;
mod hex_conv;
mod input;
mod slice_ext;
mod ui;
mod views;

use crate::app::App;
use args::Args;
use clap::Parser;
use egui_inspect::derive::Inspect;
use egui_sfml::{egui, SfEgui};
use gamedebug_core::per_msg;
use sfml::{
    graphics::{Color, Font, PrimitiveType, RenderStates, RenderTarget, RenderWindow},
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

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
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
    let mut app = App::new(args)?;

    while window.is_open() {
        do_frame(&mut app, &mut sf_egui, &mut window, &font);
    }
    Ok(())
}

fn do_frame(app: &mut App, sf_egui: &mut SfEgui, window: &mut RenderWindow, font: &Font) {
    handle_events(app, window, sf_egui);
    update(app);
    app.clamp_view();
    ui::do_egui(sf_egui, app);
    let [r, g, b] = app.bg_color;
    window.clear(Color::rgb(
        (r * 255.) as u8,
        (g * 255.) as u8,
        (b * 255.) as u8,
    ));
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
    app.vertices.clear();
    // The offset for the hex display imposed by the view
    let view_idx_off_x: usize = app.view_x.try_into().unwrap_or(0) / app.col_width as usize;
    let view_idx_off_y: usize = app.view_y.try_into().unwrap_or(0) / app.row_height as usize;
    if app.show_hex {
        views::hex(view_idx_off_y, app, view_idx_off_x, window, font);
    }
    if app.show_text {
        views::ascii(app, view_idx_off_y, window, font);
    }
    let mut rs = RenderStates::default();
    rs.set_texture(Some(font.texture(app.font_size)));
    window.draw_primitives(&app.vertices, PrimitiveType::QUADS, &rs);
    if app.show_block {
        views::block(app, view_idx_off_y, window);
    }
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
                        if shift {
                            app.view_y *= 2;
                            app.view_x /= 2;
                            app.view.cols /= 2;
                        } else {
                            app.dec_cols();
                        }
                    }
                }
                Key::Right => {
                    if app.interact_mode == InteractMode::Edit && app.cursor + 1 < app.data.len() {
                        app.cursor += 1;
                    } else if ctrl {
                        if shift {
                            app.view_y /= 2;
                            app.view.cols *= 2;
                        } else {
                            app.inc_cols();
                        }
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
                                        app.widen_dirty_region(app.cursor, None);
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
                            app.widen_dirty_region(app.cursor, None);
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
