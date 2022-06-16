#![feature(let_chains, lint_reasons, label_break_value, let_else)]

mod app;
mod args;
mod color;
mod damage_region;
mod hex_conv;
mod input;
mod slice_ext;
mod timer;
mod ui;
mod views;

use crate::app::App;
use args::Args;
use clap::Parser;
use damage_region::DamageRegion;
use egui_sfml::SfEgui;
use sfml::{
    graphics::{Color, Font, PrimitiveType, RenderStates, RenderTarget, RenderWindow},
    system::Vector2,
    window::{mouse, ContextSettings, Event, Key, Style},
};

fn msg_if_fail(result: anyhow::Result<()>, prefix: &str) {
    if let Err(e) = result {
        rfd::MessageDialog::new()
            .set_level(rfd::MessageLevel::Error)
            .set_title("Error")
            .set_description(&format!("{}: {}", prefix, e))
            .show();
    }
}

fn msg_warn(msg: &str) {
    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Warning)
        .set_title("Warning")
        .set_description(msg)
        .show();
}

#[derive(PartialEq, Eq, Debug)]
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
#[derive(PartialEq, Eq, Debug)]
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

#[derive(Clone, Copy, Debug)]
pub struct Region {
    begin: usize,
    end: usize,
}

fn main() -> anyhow::Result<()> {
    let mut args = Args::parse();
    // Streaming sources should be read-only.
    // Opening them as write blocks at EOF, which we don't want.
    if args.stream {
        args.read_only = true;
    }
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
    let mut app = App::new(args, window.size().y)?;

    while window.is_open() {
        do_frame(&mut app, &mut sf_egui, &mut window, &font);
    }
    Ok(())
}

fn do_frame(app: &mut App, sf_egui: &mut SfEgui, window: &mut RenderWindow, font: &Font) {
    handle_events(app, window, sf_egui);
    update(app);
    app.clamp_view();
    ui::do_egui(sf_egui, app, window.mouse_position());
    let [r, g, b] = app.presentation.bg_color;
    window.clear(Color::rgb(
        (r * 255.) as u8,
        (g * 255.) as u8,
        (b * 255.) as u8,
    ));
    draw(app, window, font);
    sf_egui.draw(window, None);
    window.display();
    // Should only be true on the frame right after reloading
    app.just_reloaded = false;
    gamedebug_core::inc_frame();
}

fn update(app: &mut App) {
    if app.args.stream {
        app.try_read_stream();
    }
    if app.data.is_empty() {
        return;
    }
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
                code,
                shift,
                ctrl,
                alt,
                ..
            } => handle_key_events(code, app, ctrl, shift, alt),
            Event::TextEntered { unicode } => handle_text_entered(app, unicode),
            Event::MouseButtonPressed { button, x, y } if !wants_pointer => {
                if button == mouse::Button::Left {
                    let off = app.pixel_pos_byte_offset(x, y);
                    app.set_cursor(off);
                }
            }
            Event::LostFocus => {
                // When alt-tabbing, keys held down can get "stuck", because the key release events won't reach us
                app.input.clear();
            }
            _ => {}
        }
    }
}

fn handle_text_entered(app: &mut App, unicode: char) {
    if Key::LControl.is_pressed() || Key::LAlt.is_pressed() {
        return;
    }
    match app.interact_mode {
        InteractMode::Edit => match app.edit_target {
            EditTarget::Hex => {
                if unicode.is_ascii() {
                    let ascii = unicode as u8;
                    if matches!(ascii, b'0'..=b'9' | b'a'..=b'f') {
                        match app.hex_edit_half_digit {
                            Some(half) => {
                                app.data[app.cursor] = hex_conv::merge_hex_halves(half, ascii);
                                app.widen_dirty_region(DamageRegion::Single(app.cursor()));
                                if app.cursor() + 1 < app.data.len() {
                                    app.step_cursor_forward();
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
                    app.widen_dirty_region(DamageRegion::Single(app.cursor()));
                    if app.cursor() + 1 < app.data.len() {
                        app.step_cursor_forward()
                    }
                }
            }
        },
        InteractMode::View => {}
    }
}

fn handle_key_events(code: Key, app: &mut App, ctrl: bool, shift: bool, alt: bool) {
    if app.data.is_empty() {
        return;
    }
    match code {
        Key::Up => match app.interact_mode {
            InteractMode::View => {
                if ctrl {
                    app.view.start_offset = app.view.start_offset.saturating_sub(1);
                }
            }
            InteractMode::Edit => {
                app.set_cursor_no_history(app.cursor().saturating_sub(app.view.cols));
            }
        },
        Key::Down => match app.interact_mode {
            InteractMode::View => {
                if ctrl {
                    app.view.start_offset += 1;
                }
            }
            InteractMode::Edit => {
                if app.cursor() + app.view.cols < app.data.len() {
                    app.offset_cursor(app.view.cols);
                }
            }
        },
        Key::Left => 'block: {
            if alt {
                app.cursor_history_back();
                break 'block;
            }
            if app.interact_mode == InteractMode::Edit {
                app.step_cursor_back();
            } else if ctrl {
                if shift {
                    app.halve_cols();
                } else {
                    app.dec_cols();
                }
            }
        }
        Key::Right => 'block: {
            if alt {
                app.cursor_history_forward();
                break 'block;
            }
            if app.interact_mode == InteractMode::Edit && app.cursor() + 1 < app.data.len() {
                app.step_cursor_forward();
            } else if ctrl {
                if shift {
                    app.double_cols();
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
                        app.set_cursor_no_history(app.cursor().saturating_sub(amount));
                    }
                } else {
                    app.view.start_offset = 0
                }
            }
        },
        Key::PageDown => match app.interact_mode {
            InteractMode::View => {
                let view_area = app.view_area();
                app.view_y += view_area;
                let data_h = app.data_height();
                if app.view_y + view_area > data_h {
                    app.view_y = data_h - view_area;
                }
            }
            InteractMode::Edit => {
                let amount = app.view.rows * app.view.cols;
                if app.view.start_offset + amount < app.data.len() {
                    app.view.start_offset += amount;
                    if app.interact_mode == InteractMode::Edit
                        && app.cursor() + amount < app.data.len()
                    {
                        app.offset_cursor(amount);
                    }
                }
            }
        },
        Key::Home => match app.interact_mode {
            InteractMode::View => {
                app.view_x = -10;
                app.view_y = -app.top_gap - 10;
            }
            InteractMode::Edit => {
                app.view.start_offset = 0;
                app.set_cursor_no_history(0)
            }
        },
        Key::End => match app.interact_mode {
            InteractMode::View => {
                app.center_view_on_offset(app.data.len() - 1);
            }
            InteractMode::Edit => {
                let pos = app.data.len() - app.view.rows * app.view.cols;
                app.view.start_offset = pos;
                if app.interact_mode == InteractMode::Edit {
                    app.set_cursor_no_history(pos);
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
            app.ui.find_dialog.open ^= true;
        }
        Key::S if ctrl => {
            if app.args.read_only {
                msg_warn("File opened as read-only");
            } else {
                msg_if_fail(app.save(), "Failed to save");
            }
        }
        Key::R if ctrl => {
            msg_if_fail(app.reload(), "Failed to reload");
        }
        _ => {}
    }
}
