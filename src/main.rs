#![feature(lint_reasons, label_break_value, let_else)]

mod app;
mod args;
mod color;
mod config;
mod damage_region;
mod hex_conv;
mod input;
mod metafile;
mod region;
mod slice_ext;
mod source;
mod timer;
mod ui;
mod view;

use std::{
    ffi::OsStr,
    io::{Read, Write},
};

use crate::app::App;
use app::interact_mode::InteractMode;
use args::Args;
use clap::Parser;
use config::Config;
use damage_region::DamageRegion;
use egui_sfml::SfEgui;
use gamedebug_core::imm_msg;
use interprocess::local_socket::{LocalSocketListener, LocalSocketStream};
use serde::{Deserialize, Serialize};
use sfml::{
    graphics::{Color, Font, Rect, RenderTarget, RenderWindow, Vertex, View},
    system::Vector2,
    window::{mouse, ContextSettings, Event, Key, Style, VideoMode},
};
use view::ViewKind;

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

#[derive(Serialize, Deserialize, Debug)]
struct InstanceRequest {
    args: Args,
}

fn try_main(sock_path: &OsStr) -> anyhow::Result<()> {
    let mut args = Args::parse();
    if args.instance {
        match LocalSocketStream::connect(sock_path) {
            Ok(mut stream) => {
                let vec = rmp_serde::to_vec(&InstanceRequest { args })?;
                stream.write_all(&vec)?;
                return Ok(());
            }
            Err(e) => {
                eprintln!("Failed to connect to instance: {}", e);
            }
        }
    }
    let listener = LocalSocketListener::bind(sock_path)?;
    listener.set_nonblocking(true)?;
    // Streaming sources should be read-only.
    // Opening them as write blocks at EOF, which we don't want.
    if args.stream {
        args.read_only = true;
    }
    let desktop_mode = VideoMode::desktop_mode();
    let mut window = RenderWindow::new(
        desktop_mode,
        "Hexerator",
        Style::RESIZE,
        &ContextSettings::default(),
    );
    window.set_vertical_sync_enabled(true);
    window.set_position(Vector2::new(0, 0));
    let mut sf_egui = SfEgui::new(&window);
    let font = unsafe { Font::from_memory(include_bytes!("../DejaVuSansMono.ttf")).unwrap() };
    let mut app = App::new(args, window.size().y, Config::load_or_default())?;
    let mut vertex_buffer = Vec::new();

    while window.is_open() {
        if let Ok(mut stream) = listener.accept() {
            let mut buf = Vec::new();
            stream.read_to_end(&mut buf)?;
            let req: InstanceRequest = rmp_serde::from_slice(&buf)?;
            app = App::new(req.args, window.size().y, Config::load_or_default())?;
            window.request_focus();
        }
        do_frame(
            &mut app,
            &mut sf_egui,
            &mut window,
            &font,
            &mut vertex_buffer,
        );
    }
    app.close_file();
    app.cfg.save();
    Ok(())
}

fn main() {
    let sock_path = std::env::temp_dir().join("hexerator.sock");
    if let Err(e) = try_main(sock_path.as_os_str()) {
        eprintln!("Fatal error: {}", e);
    }
    let _ = std::fs::remove_file(&sock_path);
}

fn do_frame(
    app: &mut App,
    sf_egui: &mut SfEgui,
    window: &mut RenderWindow,
    font: &Font,
    vertex_buffer: &mut Vec<Vertex>,
) {
    handle_events(app, window, sf_egui);
    update(app);
    ui::do_egui(sf_egui, app, window.mouse_position());
    let [r, g, b] = app.presentation.bg_color;
    window.clear(Color::rgb(
        (r * 255.) as u8,
        (g * 255.) as u8,
        (b * 255.) as u8,
    ));
    draw(app, window, font, vertex_buffer);
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
        let Some(key) = app.focused_view else { return };
        let spd = if app.input.key_down(Key::LShift) {
            10
        } else {
            1
        };
        if app.input.key_down(Key::Left) {
            app.views[key].scroll_offset.col_x =
                app.views[key].scroll_offset.col_x.saturating_sub(spd);
        } else if app.input.key_down(Key::Right) {
            app.views[key].scroll_offset.col_x += spd;
        }
        if app.input.key_down(Key::Up) {
            app.views[key].scroll_offset.row_y =
                app.views[key].scroll_offset.row_y.saturating_sub(spd);
        } else if app.input.key_down(Key::Down) {
            app.views[key].scroll_offset.row_y += spd;
        }
    }
}

fn draw(app: &mut App, window: &mut RenderWindow, font: &Font, vertex_buffer: &mut Vec<Vertex>) {
    let views = std::mem::take(&mut app.views);
    for (k, view) in views.iter().enumerate() {
        view.draw(k, app, window, vertex_buffer, font);
    }
    app.views = views;
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
                    app.edit_state.set_cursor(off);
                }
            }
            Event::LostFocus => {
                // When alt-tabbing, keys held down can get "stuck", because the key release events won't reach us
                app.input.clear();
            }
            Event::Resized { width, height } => {
                window.set_view(&View::from_rect(&Rect::new(
                    0.,
                    0.,
                    width as f32,
                    height as f32,
                )));
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
        InteractMode::Edit => {
            let Some(focused) = app.focused_view else {
                return
            };
            let view = &app.views[focused];
            match view.kind {
                ViewKind::Hex => {
                    if unicode.is_ascii() {
                        let ascii = unicode as u8;
                        if matches!(ascii, b'0'..=b'9' | b'a'..=b'f') {
                            match app.edit_state.hex_edit_half_digit {
                                Some(half) => {
                                    app.data[app.edit_state.cursor] =
                                        hex_conv::merge_hex_halves(half, ascii);
                                    app.widen_dirty_region(DamageRegion::Single(
                                        app.edit_state.cursor,
                                    ));
                                    if app.edit_state.cursor + 1 < app.data.len() {
                                        app.edit_state.step_cursor_forward();
                                    }
                                    app.edit_state.hex_edit_half_digit = None;
                                }
                                None => app.edit_state.hex_edit_half_digit = Some(ascii),
                            }
                        }
                    }
                }
                ViewKind::Ascii => {
                    if unicode.is_ascii() {
                        app.data[app.edit_state.cursor] = unicode as u8;
                        app.widen_dirty_region(DamageRegion::Single(app.edit_state.cursor));
                        if app.edit_state.cursor + 1 < app.data.len() {
                            app.edit_state.step_cursor_forward()
                        }
                    }
                }
                ViewKind::Block => {}
            }
        }
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
                    app.perspective.region.begin = app.perspective.region.begin.saturating_sub(1);
                }
            }
            InteractMode::Edit => {
                app.edit_state.set_cursor_no_history(
                    app.edit_state.cursor.saturating_sub(app.perspective.cols),
                );
            }
        },
        Key::Down => match app.interact_mode {
            InteractMode::View => {
                if ctrl {
                    app.perspective.region.begin += 1;
                }
            }
            InteractMode::Edit => {
                if app.edit_state.cursor + app.perspective.cols < app.data.len() {
                    app.edit_state.offset_cursor(app.perspective.cols);
                }
            }
        },
        Key::Left => 'block: {
            if alt {
                app.cursor_history_back();
                break 'block;
            }
            if app.interact_mode == InteractMode::Edit {
                app.edit_state.step_cursor_back();
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
            if app.interact_mode == InteractMode::Edit && app.edit_state.cursor + 1 < app.data.len()
            {
                app.edit_state.step_cursor_forward();
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
                // TODO: Implement properly
                //app.view_y -= 1040;
            }
            InteractMode::Edit => {
                // TODO: Implement
            }
        },
        Key::PageDown => match app.interact_mode {
            InteractMode::View => {
                todo!()
            }
            InteractMode::Edit => {
                // TODO: Implement
            }
        },
        Key::Home => match app.interact_mode {
            InteractMode::View => {
                todo!()
            }
            InteractMode::Edit => {
                app.perspective.region.begin = 0;
                app.edit_state.set_cursor_no_history(0)
            }
        },
        Key::End => match app.interact_mode {
            InteractMode::View => {
                app.center_view_on_offset(app.data.len() - 1);
            }
            InteractMode::Edit => {
                // TODO: Implement
            }
        },
        Key::Tab if shift => {
            if let Some(idx) = &mut app.focused_view {
                if *idx + 1 < app.views.len() {
                    *idx += 1;
                } else {
                    *idx = 0;
                }
            }
            app.edit_state.hex_edit_half_digit = None;
        }
        Key::F1 => app.interact_mode = InteractMode::View,
        Key::F2 => app.interact_mode = InteractMode::Edit,
        Key::F12 => app.toggle_debug(),
        Key::Escape => {
            app.edit_state.hex_edit_half_digit = None;
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
