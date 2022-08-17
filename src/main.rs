#![feature(
    lint_reasons,
    label_break_value,
    let_else,
    try_blocks,
    array_chunks,
    is_some_with
)]
#![warn(
    trivial_casts,
    trivial_numeric_casts,
    clippy::unwrap_used,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]

mod app;
mod args;
mod color;
mod config;
mod damage_region;
mod dec_conv;
pub mod edit_buffer;
mod hex_conv;
mod input;
mod metafile;
mod region;
mod shell;
mod slice_ext;
mod source;
mod timer;
mod ui;
mod view;

use std::{
    ffi::OsStr,
    fmt::Display,
    io::{Read, Write},
    path::Path,
};

use crate::{app::App, view::ViewportVec};
use anyhow::Context;
use app::{interact_mode::InteractMode, NamedView};
use args::Args;
use clap::Parser;
use config::Config;
use egui_sfml::SfEgui;
use gamedebug_core::imm_msg;
use interprocess::local_socket::{LocalSocketListener, LocalSocketStream};
use rfd::MessageButtons;
use serde::{Deserialize, Serialize};
use sfml::{
    graphics::{Color, Font, Rect, RenderTarget, RenderWindow, Vertex, View},
    system::Vector2,
    window::{mouse, ContextSettings, Event, Key, Style, VideoMode},
};
use shell::{msg_if_fail, msg_warn};
use ui::dialogs::SetCursorDialog;
use view::COMFY_MARGIN;

#[derive(Serialize, Deserialize, Debug)]
struct InstanceRequest {
    args: Args,
}

fn try_main(sock_path: &OsStr) -> anyhow::Result<()> {
    let args = Args::parse();
    if args.instance {
        match LocalSocketStream::connect(sock_path) {
            Ok(mut stream) => {
                let result: anyhow::Result<()> = try {
                    let vec = rmp_serde::to_vec(&InstanceRequest { args: args.clone() })?;
                    stream.write_all(&vec)?;
                };
                match result {
                    Ok(()) => return Ok(()),
                    Err(e) => {
                        let ans = rfd::MessageDialog::new()
                            .set_level(rfd::MessageLevel::Error)
                            .set_buttons(MessageButtons::YesNo)
                            .set_title("Hexerator")
                            .set_description(&format!(
                                "Failed to connect to instance: {}\nOpen a new instance?",
                                e
                            ))
                            .show();
                        if !ans {
                            anyhow::bail!("Failed to connect to instance");
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to connect to instance: {}", e);
            }
        }
    }
    let listener = match LocalSocketListener::bind(sock_path) {
        Ok(listener) => listener,
        Err(e) => {
            msg_warn(&format!(
                "Failed to bind IPC listener: {}\nGoing to try again.",
                e
            ));
            let _ = std::fs::remove_file(&sock_path);
            LocalSocketListener::bind(sock_path)?
        }
    };
    listener.set_nonblocking(true)?;
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
    let font = unsafe {
        Font::from_memory(include_bytes!("../DejaVuSansMono.ttf")).context("Failed to load font")?
    };
    let mut app = App::new(args, Config::load_or_default()?, &font)?;
    let mut vertex_buffer = Vec::new();

    while window.is_open() {
        if let Ok(mut stream) = listener.accept() {
            let mut buf = Vec::new();
            stream.read_to_end(&mut buf)?;
            let req: InstanceRequest = rmp_serde::from_slice(&buf)?;
            app = App::new(req.args, app.cfg, &font)?;
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
    app.cfg.save()?;
    Ok(())
}

struct SocketRemoveGuard<'a> {
    sock_path: &'a Path,
}
impl<'a> Drop for SocketRemoveGuard<'a> {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(self.sock_path);
    }
}

fn main() {
    let sock_path = std::env::temp_dir().join("hexerator.sock");
    let _guard = SocketRemoveGuard {
        sock_path: &sock_path,
    };
    msg_if_fail(try_main(sock_path.as_os_str()), "Fatal error");
}

fn do_frame(
    app: &mut App,
    sf_egui: &mut SfEgui,
    window: &mut RenderWindow,
    font: &Font,
    vertex_buffer: &mut Vec<Vertex>,
) {
    handle_events(app, window, sf_egui, font);
    update(app);
    app.update();
    let mp: ViewportVec = try_conv_mp_panic(window.mouse_position());
    ui::do_egui(sf_egui, app, mp, font);
    let [r, g, b] = app.presentation.bg_color;
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "These should be in 0-1 range, and it's just bg color. Not that important."
    )]
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
    imm_msg!(&app.perspectives);
    imm_msg!(&app.hex_iface_rect);
    imm_msg!(&app.named_views);
    gamedebug_core::inc_frame();
}

/// Try to convert mouse position to ViewportVec, show error message and panic if it fails.
///
/// Extremly high (>32700) mouse positions are unsupported.
fn try_conv_mp_panic<T: TryInto<ViewportVec>>(src: T) -> ViewportVec
where
    T::Error: Display,
{
    match src.try_into() {
        Ok(mp) => mp,
        Err(e) => {
            msg_warn(&format!("Mouse position conversion error: {}\nHexerator doesn't support extremely high (>32700) mouse positions.", e));
            panic!("Mouse position conversion error.");
        }
    }
}

fn update(app: &mut App) {
    app.try_read_stream();
    if app.data.is_empty() {
        return;
    }
    if app.interact_mode == InteractMode::View && !app.input.key_down(Key::LControl) {
        let Some(idx) = app.focused_view else { return };
        let spd = if app.input.key_down(Key::LShift) {
            10
        } else {
            1
        };
        if app.input.key_down(Key::Left) {
            app.named_views[idx].view.scroll_x(-spd);
        } else if app.input.key_down(Key::Right) {
            app.named_views[idx].view.scroll_x(spd);
        }
        if app.input.key_down(Key::Up) {
            app.named_views[idx].view.scroll_y(-spd);
        } else if app.input.key_down(Key::Down) {
            app.named_views[idx].view.scroll_y(spd);
        }
    }
    // Sync all other views to active view
    if let Some(idx) = app.focused_view {
        let src = &app.named_views[idx].view;
        if !src.active {
            app.focused_view = None;
        }
        let (src_row, src_col) = (src.scroll_offset.row(), src.scroll_offset.col());
        let (src_yoff, src_xoff) = (src.scroll_offset.pix_yoff(), src.scroll_offset.pix_xoff());
        let (src_row_h, src_col_w) = (src.row_h, src.col_w);
        for NamedView { view, name: _ } in &mut app.named_views {
            view.sync_to(src_row, src_yoff, src_col, src_xoff, src_row_h, src_col_w);
            // Also clamp view ranges
            if view.scroll_offset.row == 0 && view.scroll_offset.pix_yoff < COMFY_MARGIN {
                view.scroll_offset.pix_yoff = COMFY_MARGIN;
            }
            if view.scroll_offset.col == 0 && view.scroll_offset.pix_xoff < COMFY_MARGIN {
                view.scroll_offset.pix_xoff = COMFY_MARGIN;
            }
        }
    }
}

fn draw(app: &mut App, window: &mut RenderWindow, font: &Font, vertex_buffer: &mut Vec<Vertex>) {
    let views = std::mem::take(&mut app.named_views);
    for (k, view) in views.iter().enumerate() {
        view.view.draw(k, app, window, vertex_buffer, font);
    }
    app.named_views = views;
}

fn handle_events(app: &mut App, window: &mut RenderWindow, sf_egui: &mut SfEgui, font: &Font) {
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
            } => handle_key_events(code, app, ctrl, shift, alt, font),
            Event::TextEntered { unicode } => handle_text_entered(app, unicode),
            Event::MouseButtonPressed { button, x, y } if !wants_pointer => {
                let mp = try_conv_mp_panic((x, y));
                if button == mouse::Button::Left {
                    if let Some((off, _view_idx)) = app.byte_offset_at_pos(mp.x, mp.y) {
                        app.edit_state.set_cursor(off);
                    }
                    if let Some(view_idx) = app.view_idx_at_pos(mp.x, mp.y) {
                        app.focused_view = Some(view_idx);
                    }
                }
            }
            Event::LostFocus => {
                // When alt-tabbing, keys held down can get "stuck", because the key release events won't reach us
                app.input.clear();
            }
            Event::Resized {
                mut width,
                mut height,
            } => {
                let mut needs_window_resize = false;
                const MIN_WINDOW_W: u32 = 920;
                if width < MIN_WINDOW_W {
                    width = MIN_WINDOW_W;
                    needs_window_resize = true;
                }
                const MIN_WINDOW_H: u32 = 620;
                if height < MIN_WINDOW_H {
                    height = MIN_WINDOW_H;
                    needs_window_resize = true;
                }
                if needs_window_resize {
                    window.set_size((width, height));
                }
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "Window sizes larger than i16::MAX aren't supported."
                )]
                window.set_view(&View::from_rect(&Rect::new(
                    0.,
                    0.,
                    width as f32,
                    height as f32,
                )));
                app.resize_views.reset();
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
            let mut view = std::mem::replace(
                &mut app.named_views[focused].view,
                crate::view::View::zeroed(),
            );
            view.handle_text_entered(unicode, app);
            app.named_views[focused].view = view;
        }
        InteractMode::View => {}
    }
}

fn handle_key_events(code: Key, app: &mut App, ctrl: bool, shift: bool, alt: bool, font: &Font) {
    if code == Key::F12 && !shift && !ctrl && !alt {
        app.toggle_debug()
    }
    if app.data.is_empty() {
        return;
    }
    match code {
        Key::Up => match app.interact_mode {
            InteractMode::View => {
                if ctrl && let Some(view_idx) = app.focused_view {
                    let key = app.named_views[view_idx].view.perspective;
                    app.regions[app.perspectives[key].region].region.begin = app.regions[app.perspectives[key].region].region.begin.saturating_sub(1);
                }
            }
            InteractMode::Edit => {
                if let Some(view_idx) = app.focused_view {
                    let view = &mut app.named_views[view_idx].view;
                    view.undirty_edit_buffer();
                    app.edit_state.set_cursor_no_history(
                        app.edit_state.cursor.saturating_sub(app.perspectives[view.perspective].cols),
                    );
                }
            }
        },
        Key::Down => match app.interact_mode {
            InteractMode::View => {
                if ctrl && let Some(view_idx) = app.focused_view {
                    let key = app.named_views[view_idx].view.perspective;
                    app.regions[app.perspectives[key].region].region.begin += 1;
                }
            }
            InteractMode::Edit => {
                if let Some(view_idx) = app.focused_view {
                    let view = &mut app.named_views[view_idx].view;
                    view.undirty_edit_buffer();
                    if app.edit_state.cursor + app.perspectives[view.perspective].cols < app.data.len() {
                        app.edit_state.offset_cursor(app.perspectives[view.perspective].cols);
                    }
                }
            }
        },
        Key::Left => 'block: {
            if alt {
                app.cursor_history_back();
                break 'block;
            }
            if app.interact_mode == InteractMode::Edit {
                let move_edit = (app.preferences.move_edit_cursor && !ctrl)
                    || (!app.preferences.move_edit_cursor && ctrl);
                if move_edit {
                    if let Some(view_idx) = app.focused_view {
                        let view = &mut app.named_views[view_idx];
                        if let Some(edit_buf) = view.view.edit_buffer_mut() {
                            if !edit_buf.move_cursor_back() {
                                edit_buf.move_cursor_end();
                                edit_buf.dirty = false;
                                app.edit_state.step_cursor_back();
                            }
                        }
                    }
                } else {
                    app.edit_state.step_cursor_back();
                }
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
                let move_edit = (app.preferences.move_edit_cursor && !ctrl)
                    || (!app.preferences.move_edit_cursor && ctrl);
                if move_edit {
                    if let Some(view_idx) = app.focused_view {
                        let view = &mut app.named_views[view_idx];
                        if let Some(edit_buf) = &mut view.view.edit_buffer_mut() {
                            if !edit_buf.move_cursor_forward() {
                                edit_buf.move_cursor_begin();
                                edit_buf.dirty = false;
                                app.edit_state.step_cursor_forward();
                            }
                        }
                    }
                } else {
                    app.edit_state.step_cursor_forward();
                }
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
                if let Some(idx) = app.focused_view {
                    app.named_views[idx].view.scroll_page_up();
                }
            }
            InteractMode::Edit => {
                // TODO: Implement
            }
        },
        Key::PageDown => match app.interact_mode {
            InteractMode::View => {
                if let Some(idx) = app.focused_view {
                    app.named_views[idx].view.scroll_page_down();
                }
            }
            InteractMode::Edit => {
                // TODO: Implement
            }
        },
        Key::Home => {
            if let Some(idx) = app.focused_view {
                let view = &mut app.named_views[idx].view;
                match app.interact_mode {
                    InteractMode::View => {
                        view.go_home();
                    }
                    InteractMode::Edit => {
                        app.regions[app.perspectives[view.perspective].region].region.begin = 0;
                        app.edit_state.set_cursor_no_history(0);
                    }
                }
            }
        },
        Key::End => match app.interact_mode {
            InteractMode::View => {
                if let Some(idx) = app.focused_view {
                    app.named_views[idx].view.scroll_to_end(&app.perspectives, &app.regions);
                }
            }
            InteractMode::Edit => {
                // TODO: Implement
            }
        },
        Key::Tab if shift => {
            if let Some(idx) = &mut app.focused_view {
                if *idx + 1 < app.named_views.len() {
                    *idx += 1;
                } else {
                    *idx = 0;
                }
            }
        }
        Key::F1 => app.interact_mode = InteractMode::View,
        Key::F2 => app.interact_mode = InteractMode::Edit,
        Key::F5 => app.ui.views_window.open ^= true,
        Key::Escape => {
            if let Some(view_idx) = app.focused_view {
                app.named_views[view_idx].view.cancel_editing();
            }
            app.select_a = None;
            app.select_b = None;
        }
        Key::Enter => {
            if let Some(view_idx) = app.focused_view {
                let mut view = std::mem::replace(
                    &mut app.named_views[view_idx].view,
                    crate::view::View::zeroed(),
                );
                view.finish_editing(app);
                app.named_views[view_idx].view = view;
            }
        }
        Key::F if ctrl => {
            app.ui.find_dialog.open ^= true;
        }
        Key::S if ctrl => match &mut app.source {
            Some(source) => {
                if !source.attr.permissions.write {
                    msg_warn("This source cannot be written to.");
                } else {
                    msg_if_fail(app.save(), "Failed to save");
                }
            }
            None => msg_warn("No source opened"),
        },
        Key::R if ctrl => {
            msg_if_fail(app.reload(), "Failed to reload");
        }
        Key::O if ctrl => {
            shell::open_file(app, font);
        }
        Key::W if ctrl => app.close_file(),
        Key::J if ctrl => app.ui.add_dialog(SetCursorDialog::default()),
        Key::Num1 if shift => app.select_a = Some(app.edit_state.cursor),
        Key::Num2 if shift => app.select_b = Some(app.edit_state.cursor),
        _ => {}
    }
}
