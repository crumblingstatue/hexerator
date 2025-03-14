#![doc(html_no_source)]
#![feature(
    try_blocks,
    array_chunks,
    let_chains,
    array_windows,
    generic_const_exprs,
    macro_metavar_expr_concat,
    default_field_values,
    yeet_expr
)]
#![warn(
    trivial_casts,
    trivial_numeric_casts,
    unsafe_op_in_unsafe_fn,
    clippy::unwrap_used,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::panic,
    clippy::needless_pass_by_ref_mut,
    clippy::semicolon_if_nothing_returned,
    clippy::items_after_statements,
    clippy::unused_trait_names,
    clippy::undocumented_unsafe_blocks
)]
#![expect(
    incomplete_features,
    // It's hard to reconcile lack of partial borrows with few arguments
    clippy::too_many_arguments
)]
#![windows_subsystem = "windows"]

use {
    crate::{app::App, view::ViewportVec},
    anyhow::Context as _,
    app::interact_mode::InteractMode,
    args::Args,
    clap::Parser as _,
    config::{Config, LoadedConfig, ProjectDirsExt as _},
    core::f32,
    egui_colors::{Colorix, tokens::ThemeColor},
    egui_file_dialog::{DialogState, DirectoryEntry, NativeFileSystem},
    egui_sfml::{
        SfEgui,
        sfml::{
            graphics::{
                Color, Font, Rect, RenderStates, RenderTarget as _, RenderWindow, Text,
                Transformable as _, Vertex, View,
            },
            system::Vector2,
            window::{ContextSettings, Event, Key, Style, VideoMode, mouse},
        },
    },
    gamedebug_core::{IMMEDIATE, PERSISTENT, per},
    gui::{
        Gui,
        command::GCmd,
        dialogs::JumpDialog,
        message_dialog::{Icon, MessageDialog},
        root_ctx_menu::{ContextMenu, ContextMenuData},
    },
    meta::{NamedView, PerspectiveMap, RegionMap},
    mlua::Lua,
    shell::msg_if_fail,
    slotmap::Key as _,
    std::{
        backtrace::{Backtrace, BacktraceStatus},
        fmt::Display,
        io::IsTerminal as _,
        time::Duration,
    },
};

mod app;
mod args;
mod backend;
mod color;
mod config;
mod damage_region;
mod data;
mod dec_conv;
pub mod edit_buffer;
mod find_util;
mod gui;
mod hex_conv;
mod hex_ui;
mod input;
mod layout;
mod meta;
mod meta_state;
mod parse_radix;
mod plugin;
mod preferences;
mod scripting;
mod shell;
mod slice_ext;
mod source;
mod str_ext;
mod struct_meta_item;
mod timer;
mod util;
mod value_color;
mod view;
#[cfg(windows)]
mod windows;

fn print_version_info() {
    eprintln!(
        "Hexerator {} ({} {}), built on {}",
        env!("CARGO_PKG_VERSION"),
        env!("VERGEN_GIT_SHA"),
        env!("VERGEN_GIT_COMMIT_TIMESTAMP"),
        env!("VERGEN_BUILD_TIMESTAMP")
    );
}

fn try_main() -> anyhow::Result<()> {
    let mut args = Args::parse();
    if args.debug {
        IMMEDIATE.set_enabled(true);
        PERSISTENT.set_enabled(true);
    }
    if args.version {
        print_version_info();
        return Ok(());
    }
    let desktop_mode = VideoMode::desktop_mode();
    let mut window = RenderWindow::new(
        desktop_mode,
        "Hexerator",
        Style::RESIZE | Style::CLOSE,
        &ContextSettings::default(),
    )?;
    let LoadedConfig {
        config: mut cfg,
        old_config_err,
    } = Config::load_or_default()?;
    window.set_vertical_sync_enabled(cfg.vsync);
    window.set_framerate_limit(cfg.fps_limit);
    window.set_position(Vector2::new(0, 0));
    let mut sf_egui = SfEgui::new(&window);
    sf_egui.context().options_mut(|opts| {
        opts.zoom_with_keyboard = false;
    });
    let mut style = egui::Style::default();
    style.interaction.show_tooltips_only_when_still = true;
    let font = Font::from_memory_static(include_bytes!("../DejaVuSansMono.ttf"))
        .context("Failed to load font")?;
    let mut gui = Gui::default();
    gui.win.open_process.default_meta_path.clone_from(&args.meta);
    transfer_pinned_folders_to_file_dialog(&mut gui, &mut cfg);
    if !args.spawn_command.is_empty() {
        gui.cmd.push(GCmd::SpawnCommand {
            args: std::mem::take(&mut args.spawn_command),
            look_for_proc: args.look_for_proc.take(),
        });
    }
    if let Some(e) = old_config_err {
        gui.msg_dialog.open(
            Icon::Error,
            "Failed to load old config",
            format!("Old config failed to load with error: {e}.\n\
                     If you don't want to overwrite the old config, you should probably not continue."),
        );
        gui.msg_dialog.custom_button_row_ui(Box::new(|ui, modal, _cmd| {
            if ui.button("⚠️ Continue").clicked() {
                modal.is_open = false;
            }
            if ui.button("Abort").clicked() {
                std::process::abort();
            }
        }));
    }
    let mut font_defs = egui::FontDefinitions::default();
    egui_fontcfg::load_custom_fonts(&cfg.custom_font_paths, &mut font_defs.font_data)?;
    if !cfg.font_families.is_empty() {
        font_defs.families = cfg.font_families.clone();
    }
    sf_egui.context().set_fonts(font_defs);
    let font_size = 14;
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "It's extremely unlikely that the line spacing is not between 0..u16::MAX"
    )]
    let line_spacing = font.line_spacing(u32::from(font_size)) as u16;
    let mut app = App::new(args, cfg, font_size, line_spacing, &mut gui.msg_dialog)?;
    let lua = Lua::default();
    crate::gui::set_font_sizes_style(&mut style, &app.cfg.style);
    sf_egui.context().set_style(style);
    // Custom egui_colors theme load
    if let Some(project_dirs) = crate::config::project_dirs() {
        let path = project_dirs.color_theme_path();
        if path.exists() {
            match std::fs::read(path) {
                Ok(data) => {
                    let mut chunks = data.array_chunks().copied();
                    let theme = std::array::from_fn(|_| {
                        ThemeColor::Custom(chunks.next().unwrap_or_default())
                    });
                    gui.colorix = Some(Colorix::global(sf_egui.context(), theme));
                }
                Err(e) => {
                    eprintln!("Failed to load custom theme: {e}");
                }
            }
        }
    }
    let mut vertex_buffer = Vec::new();

    while window.is_open() {
        if !do_frame(
            &mut app,
            &mut gui,
            &mut sf_egui,
            &mut window,
            &mut vertex_buffer,
            &lua,
            &font,
        )? {
            return Ok(());
        }
        // Save a metafile backup every so often
        if app.meta_state.last_meta_backup.get().elapsed() >= Duration::from_secs(60) {
            if let Err(e) = app.save_temp_metafile_backup() {
                per!("Failed to save temp metafile backup: {}", e);
            }
        }
    }
    app.close_file();
    transfer_pinned_folders_to_config(gui, &mut app);
    app.cfg.save()?;
    Ok(())
}

fn transfer_pinned_folders_to_file_dialog(gui: &mut Gui, cfg: &mut Config) {
    let dia_cfg = gui.fileops.dialog.config_mut();
    // Remove them from the config, as later it will be filled with
    // the pinned dirs from the dialog
    for dir in cfg.pinned_dirs.drain(..) {
        dia_cfg.storage.pinned_folders.push(DirectoryEntry::from_path(
            dia_cfg,
            &dir,
            &NativeFileSystem,
        ));
    }
}

fn transfer_pinned_folders_to_config(mut gui: Gui, app: &mut App) {
    let storage = gui.fileops.dialog.storage_mut();
    for entry in &storage.pinned_folders {
        app.cfg.pinned_dirs.push(entry.to_path_buf());
    }
}

fn main() {
    std::panic::set_hook(Box::new(|panic_info| {
        let payload = panic_info.payload();
        let msg = if let Some(s) = payload.downcast_ref::<&str>() {
            s
        } else if let Some(s) = payload.downcast_ref::<String>() {
            s
        } else {
            "Unknown panic payload"
        };
        let (file, line, column) = match panic_info.location() {
            Some(loc) => (loc.file(), loc.line().to_string(), loc.column().to_string()),
            None => ("unknown", "unknown".into(), "unknown".into()),
        };
        let bkpath = app::temp_metafile_backup_path();
        let bkpath = bkpath.display();
        let btrace = Backtrace::force_capture();
        do_fatal_error_report(
            "Hexerator panic",
            &format!(
                "\
            {msg}\n\n\
            Location:\n\
            {file}:{line}:{column}\n\n\
            Meta Backup path:\n\
            {bkpath}",
            ),
            &btrace,
        );
    }));
    if let Err(e) = try_main() {
        do_fatal_error_report("Fatal error", &e.to_string(), e.backtrace());
    }
}

fn do_fatal_error_report(title: &str, mut desc: &str, backtrace: &Backtrace) {
    if std::io::stderr().is_terminal() {
        eprintln!("== {title} ==");
        eprintln!("{desc}");
        if backtrace.status() == BacktraceStatus::Captured {
            eprintln!("Backtrace:\n{backtrace}");
        }
        return;
    }
    let bt_string = if backtrace.status() == BacktraceStatus::Captured {
        backtrace.to_string()
    } else {
        String::new()
    };
    let mut rw =
        match RenderWindow::new((800, 600), title, Style::CLOSE, &ContextSettings::default()) {
            Ok(rw) => rw,
            Err(e) => {
                eprintln!("Failed to create RenderWindow: {e}");
                return;
            }
        };
    rw.set_vertical_sync_enabled(true);
    let mut sf_egui = SfEgui::new(&rw);
    while rw.is_open() {
        while let Some(ev) = rw.poll_event() {
            sf_egui.add_event(&ev);
            if ev == Event::Closed {
                rw.close();
            }
        }
        rw.clear(Color::BLACK);
        #[expect(clippy::unwrap_used)]
        let di = sf_egui
            .run(&mut rw, |rw, ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.heading(title);
                    ui.separator();
                    egui::ScrollArea::vertical().auto_shrink(false).max_height(500.).show(
                        ui,
                        |ui| {
                            ui.add(
                                egui::TextEdit::multiline(&mut desc)
                                    .code_editor()
                                    .desired_width(f32::INFINITY),
                            );
                            if !bt_string.is_empty() {
                                ui.heading("Backtrace");
                                ui.add(
                                    egui::TextEdit::multiline(&mut bt_string.as_str())
                                        .code_editor()
                                        .desired_width(f32::INFINITY),
                                );
                            }
                        },
                    );
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("Copy to clipboard").clicked() {
                            ctx.copy_text(desc.to_owned());
                        }
                        if ui.button("Close").clicked() {
                            rw.close();
                        }
                    });
                });
            })
            .unwrap();
        sf_egui.draw(di, &mut rw, None);
        rw.display();
    }
}

#[must_use = "Returns false if application should quit"]
fn do_frame(
    app: &mut App,
    gui: &mut Gui,
    sf_egui: &mut SfEgui,
    window: &mut RenderWindow,
    vertex_buffer: &mut Vec<Vertex>,
    lua: &Lua,
    font: &Font,
) -> anyhow::Result<bool> {
    let font_size = 14;
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "It's extremely unlikely that the line spacing is not between 0..u16::MAX"
    )]
    let line_spacing = font.line_spacing(u32::from(font_size)) as u16;
    // Handle window events
    handle_events(gui, app, window, sf_egui, font_size, line_spacing);
    update(app, sf_egui.context().wants_keyboard_input());
    app.update(gui, window, lua, font_size, line_spacing);
    let mp: ViewportVec = try_conv_mp_zero(window.mouse_position());
    let (di, cont) = gui::do_egui(sf_egui, gui, app, mp, lua, window, font_size, line_spacing)?;
    if !cont {
        return Ok(false);
    }
    // Here we flush GUI command queue every frame
    gui.flush_command_queue();
    let [r, g, b] = app.preferences.bg_color;
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
    draw(app, gui, window, font, vertex_buffer);
    if let Some((offs, _view)) = app.byte_offset_at_pos(mp.x, mp.y) {
        if let Some(bm) = app.meta_state.meta.bookmarks.iter().find(|bm| bm.offset == offs) {
            let mut txt = Text::new(&bm.label, font, 20);
            txt.set_position((f32::from(mp.x), f32::from(mp.y + 15)));
            window.draw_text(&txt, &RenderStates::DEFAULT);
        }
        // Mouse drag selection
        if let Some(a) = app.hex_ui.lmb_drag_offset
            && offs != a
        {
            app.hex_ui.select_a = Some(a);
            app.hex_ui.select_b = Some(offs);
        }
    }
    sf_egui.draw(di, window, None);
    window.display();
    gamedebug_core::inc_frame();
    if app.quit_requested {
        return Ok(false);
    }
    Ok(true)
}

/// Try to convert mouse position to ViewportVec.
///
/// Log error and return zeroed vec on conversion error.
fn try_conv_mp_zero<T: TryInto<ViewportVec>>(src: T) -> ViewportVec
where
    T::Error: Display,
{
    match src.try_into() {
        Ok(mp) => mp,
        Err(e) => {
            per!(
                "Mouse position conversion error: {}\nHexerator doesn't support extremely high (>32700) mouse positions.",
                e
            );
            ViewportVec { x: 0, y: 0 }
        }
    }
}

fn update(app: &mut App, egui_wants_kb: bool) {
    app.try_read_stream();
    if app.data.is_empty() {
        return;
    }
    app.hex_ui.show_alt_overlay = app.input.key_down(Key::LAlt);
    if !egui_wants_kb
        && app.hex_ui.interact_mode == InteractMode::View
        && !app.input.key_down(Key::LControl)
    {
        let Some(key) = app.hex_ui.focused_view else {
            return;
        };
        let spd = if app.input.key_down(Key::LShift) {
            10
        } else {
            1
        };
        if app.input.key_down(Key::Left) {
            app.meta_state.meta.views[key].view.scroll_x(-spd);
        } else if app.input.key_down(Key::Right) {
            app.meta_state.meta.views[key].view.scroll_x(spd);
        }
        if app.input.key_down(Key::Up) {
            app.meta_state.meta.views[key].view.scroll_y(-spd);
        } else if app.input.key_down(Key::Down) {
            app.meta_state.meta.views[key].view.scroll_y(spd);
        }
    }
    // Sync all other views to active view
    if let Some(key) = app.hex_ui.focused_view {
        let src = &app.meta_state.meta.views[key].view;
        let src_perspective = src.perspective;
        let (src_row, src_col) = (src.scroll_offset.row(), src.scroll_offset.col());
        let (src_yoff, src_xoff) = (src.scroll_offset.pix_yoff(), src.scroll_offset.pix_xoff());
        let (src_row_h, src_col_w) = (src.row_h, src.col_w);
        for NamedView { view, name: _ } in app.meta_state.meta.views.values_mut() {
            // Only sync views that have the same perspective
            if view.perspective != src_perspective {
                continue;
            }
            view.sync_to(src_row, src_yoff, src_col, src_xoff, src_row_h, src_col_w);
            // Also clamp view ranges
            if view.scroll_offset.row == 0 && view.scroll_offset.pix_yoff < 0 {
                view.scroll_offset.pix_yoff = 0;
            }
            if view.scroll_offset.col == 0 && view.scroll_offset.pix_xoff < 0 {
                view.scroll_offset.pix_xoff = 0;
            }
            let Some(per) = &app.meta_state.meta.low.perspectives.get(view.perspective) else {
                per!("View doesn't have a perspective. Probably a bug.");
                continue;
            };
            if view.cols() < 0 {
                per!("view.cols for some reason is less than 0. Probably a bug.");
                return;
            }
            if view.scroll_offset.col + 1 > per.cols {
                view.scroll_offset.col = per.cols - 1;
                view.scroll_offset.pix_xoff = 0;
            }
            if view.scroll_offset.row + 1 > per.n_rows(&app.meta_state.meta.low.regions) {
                view.scroll_offset.row =
                    per.n_rows(&app.meta_state.meta.low.regions).saturating_sub(1);
                view.scroll_offset.pix_yoff = 0;
            }
        }
    }
}

fn draw(
    app: &App,
    gui: &Gui,
    window: &mut RenderWindow,
    font: &Font,
    vertex_buffer: &mut Vec<Vertex>,
) {
    if app.hex_ui.current_layout.is_null() {
        let mut t = Text::new("No active layout", font, 20);
        t.set_position((
            f32::from(app.hex_ui.hex_iface_rect.x),
            f32::from(app.hex_ui.hex_iface_rect.y),
        ));
        window.draw_text(&t, &RenderStates::DEFAULT);
        return;
    }
    for view_key in app.meta_state.meta.layouts[app.hex_ui.current_layout].iter() {
        crate::view::View::draw(view_key, app, gui, window, vertex_buffer, font);
    }
}

fn handle_events(
    gui: &mut crate::gui::Gui,
    app: &mut App,
    window: &mut RenderWindow,
    sf_egui: &mut SfEgui,
    font_size: u16,
    line_spacing: u16,
) {
    while let Some(event) = window.poll_event() {
        let egui_ctx = sf_egui.context();
        let wants_pointer = egui_ctx.wants_pointer_input();
        let wants_kb = egui_ctx.wants_keyboard_input()
            || matches!(gui.fileops.dialog.state(), DialogState::Open);
        let block_event_from_egui = (matches!(event, Event::KeyPressed { code: Key::Tab, .. })
            && !(wants_kb || wants_pointer));
        if !block_event_from_egui {
            sf_egui.add_event(&event);
        }
        if wants_kb {
            if event == Event::Closed {
                window.close();
            }
            app.input.clear();
            continue;
        }
        app.input.update_from_event(&event);
        match event {
            Event::Closed => window.close(),
            Event::KeyPressed {
                code,
                shift,
                ctrl,
                alt,
                ..
            } => handle_key_pressed(
                code,
                gui,
                app,
                KeyMod { ctrl, shift, alt },
                wants_kb,
                font_size,
                line_spacing,
            ),
            Event::TextEntered { unicode } => {
                handle_text_entered(app, unicode, &mut gui.msg_dialog);
            }
            Event::MouseButtonPressed { button, x, y } if !wants_pointer => {
                let mp = try_conv_mp_zero((x, y));
                if app.hex_ui.current_layout.is_null() {
                    continue;
                }
                if button == mouse::Button::Left {
                    gui.context_menu = None;
                    if let Some((off, _view_idx)) = app.byte_offset_at_pos(mp.x, mp.y) {
                        app.hex_ui.lmb_drag_offset = Some(off);
                        app.edit_state.set_cursor(off);
                    }
                    if let Some(view_idx) = app.view_idx_at_pos(mp.x, mp.y) {
                        app.hex_ui.focused_view = Some(view_idx);
                        gui.win.views.selected = view_idx;
                    }
                } else if button == mouse::Button::Right {
                    match app.view_at_pos(mp.x, mp.y) {
                        Some(view_key) => match app.view_byte_offset_at_pos(view_key, mp.x, mp.y) {
                            Some(pos) => {
                                gui.context_menu = Some(ContextMenu::new(
                                    mp.x,
                                    mp.y,
                                    ContextMenuData {
                                        view: Some(view_key),
                                        byte_off: Some(pos),
                                    },
                                ));
                            }
                            None => {
                                gui.context_menu = Some(ContextMenu::new(
                                    mp.x,
                                    mp.y,
                                    ContextMenuData {
                                        view: Some(view_key),
                                        byte_off: None,
                                    },
                                ));
                            }
                        },
                        None => {
                            gui.context_menu = Some(ContextMenu::new(
                                mp.x,
                                mp.y,
                                ContextMenuData {
                                    view: None,
                                    byte_off: None,
                                },
                            ));
                        }
                    }
                }
            }
            Event::MouseButtonReleased {
                button: mouse::Button::Left,
                ..
            } => {
                app.hex_ui.lmb_drag_offset = None;
            }
            Event::LostFocus => {
                // When alt-tabbing, keys held down can get "stuck", because the key release events won't reach us
                app.input.clear();
            }
            Event::Resized {
                mut width,
                mut height,
            } => {
                const MIN_WINDOW_W: u32 = 920;
                const MIN_WINDOW_H: u32 = 620;

                let mut needs_window_resize = false;
                if width < MIN_WINDOW_W {
                    width = MIN_WINDOW_W;
                    needs_window_resize = true;
                }
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
                match View::from_rect(Rect::new(0., 0., width as f32, height as f32)) {
                    Ok(view) => window.set_view(&view),
                    Err(e) => {
                        gamedebug_core::per!("Failed to create view: {e}");
                    }
                }
            }
            _ => {}
        }
    }
}

fn handle_text_entered(app: &mut App, unicode: char, msg: &mut MessageDialog) {
    if Key::LControl.is_pressed() || Key::LAlt.is_pressed() {
        return;
    }
    match app.hex_ui.interact_mode {
        InteractMode::Edit => {
            let Some(focused) = app.hex_ui.focused_view else {
                return;
            };
            let view = &mut app.meta_state.meta.views[focused].view;
            view.handle_text_entered(
                unicode,
                &mut app.edit_state,
                &app.preferences,
                &mut app.data,
                msg,
            );
            keep_cursor_in_view(
                view,
                &app.meta_state.meta.low.perspectives,
                &app.meta_state.meta.low.regions,
                app.edit_state.cursor,
            );
        }
        InteractMode::View => {}
    }
}

struct KeyMod {
    ctrl: bool,
    shift: bool,
    alt: bool,
}

fn handle_key_pressed(
    code: Key,
    gui: &mut crate::gui::Gui,
    app: &mut App,
    key_mod: KeyMod,
    egui_wants_kb: bool,
    font_size: u16,
    line_spacing: u16,
) {
    if code == Key::F12 && !key_mod.shift && !key_mod.ctrl && !key_mod.alt {
        IMMEDIATE.toggle();
        PERSISTENT.toggle();
    }
    if egui_wants_kb {
        return;
    }
    // Key bindings that should work without any file open
    match code {
        Key::O if key_mod.ctrl => {
            gui.fileops.load_file(app.source_file());
        }
        _ => {}
    }
    if app.data.is_empty() {
        return;
    }
    // Key bindings that should only work with a file open
    match code {
        Key::Up => match app.hex_ui.interact_mode {
            InteractMode::View => {
                if key_mod.ctrl
                    && let Some(view_key) = app.hex_ui.focused_view
                {
                    let key = app.meta_state.meta.views[view_key].view.perspective;
                    let reg = &mut app.meta_state.meta.low.regions
                        [app.meta_state.meta.low.perspectives[key].region]
                        .region;
                    reg.begin = reg.begin.saturating_sub(1);
                }
            }
            InteractMode::Edit => {
                if let Some(view_key) = app.hex_ui.focused_view {
                    let view = &mut app.meta_state.meta.views[view_key].view;
                    view.undirty_edit_buffer();
                    app.edit_state.set_cursor_no_history(app.edit_state.cursor.saturating_sub(
                        app.meta_state.meta.low.perspectives[view.perspective].cols,
                    ));
                    keep_cursor_in_view(
                        view,
                        &app.meta_state.meta.low.perspectives,
                        &app.meta_state.meta.low.regions,
                        app.edit_state.cursor,
                    );
                }
            }
        },
        Key::Down => match app.hex_ui.interact_mode {
            InteractMode::View => {
                if key_mod.ctrl
                    && let Some(view_key) = app.hex_ui.focused_view
                {
                    let key = app.meta_state.meta.views[view_key].view.perspective;
                    app.meta_state.meta.low.regions
                        [app.meta_state.meta.low.perspectives[key].region]
                        .region
                        .begin += 1;
                }
            }
            InteractMode::Edit => {
                if let Some(view_key) = app.hex_ui.focused_view {
                    let view = &mut app.meta_state.meta.views[view_key].view;
                    view.undirty_edit_buffer();
                    if app.edit_state.cursor
                        + app.meta_state.meta.low.perspectives[view.perspective].cols
                        < app.data.len()
                    {
                        app.edit_state.offset_cursor(
                            app.meta_state.meta.low.perspectives[view.perspective].cols,
                        );
                    }
                    keep_cursor_in_view(
                        view,
                        &app.meta_state.meta.low.perspectives,
                        &app.meta_state.meta.low.regions,
                        app.edit_state.cursor,
                    );
                }
            }
        },
        Key::Left => 'block: {
            if key_mod.alt {
                app.cursor_history_back();
                break 'block;
            }
            if app.hex_ui.interact_mode == InteractMode::Edit {
                let move_edit = (app.preferences.move_edit_cursor && !key_mod.ctrl)
                    || (!app.preferences.move_edit_cursor && key_mod.ctrl);
                if let Some(view_key) = app.hex_ui.focused_view {
                    let view = &mut app.meta_state.meta.views[view_key];
                    if move_edit {
                        if let Some(edit_buf) = view.view.edit_buffer_mut() {
                            if !edit_buf.move_cursor_back() {
                                edit_buf.move_cursor_end();
                                edit_buf.dirty = false;
                                app.edit_state.step_cursor_back();
                            }
                        }
                    } else {
                        app.edit_state.step_cursor_back();
                        keep_cursor_in_view(
                            &mut view.view,
                            &app.meta_state.meta.low.perspectives,
                            &app.meta_state.meta.low.regions,
                            app.edit_state.cursor,
                        );
                    }
                }
            } else if key_mod.ctrl {
                if key_mod.shift {
                    app.halve_cols();
                } else {
                    app.dec_cols();
                }
            }
        }
        Key::Right => 'block: {
            if key_mod.alt {
                app.cursor_history_forward();
                break 'block;
            }
            if app.hex_ui.interact_mode == InteractMode::Edit
                && app.edit_state.cursor + 1 < app.data.len()
            {
                let move_edit = (app.preferences.move_edit_cursor && !key_mod.ctrl)
                    || (!app.preferences.move_edit_cursor && key_mod.ctrl);
                if let Some(view_key) = app.hex_ui.focused_view {
                    let view = &mut app.meta_state.meta.views[view_key];
                    if move_edit {
                        if let Some(edit_buf) = &mut view.view.edit_buffer_mut() {
                            if !edit_buf.move_cursor_forward() {
                                edit_buf.move_cursor_begin();
                                edit_buf.dirty = false;
                                app.edit_state.step_cursor_forward();
                            }
                        }
                    } else {
                        app.edit_state.step_cursor_forward();
                        keep_cursor_in_view(
                            &mut view.view,
                            &app.meta_state.meta.low.perspectives,
                            &app.meta_state.meta.low.regions,
                            app.edit_state.cursor,
                        );
                    }
                }
            } else if key_mod.ctrl {
                if key_mod.shift {
                    app.double_cols();
                } else {
                    app.inc_cols();
                }
            }
        }
        Key::PageUp => {
            if let Some(key) = app.hex_ui.focused_view {
                let view = &mut app.meta_state.meta.views[key].view;
                let per = &app.meta_state.meta.low.perspectives[view.perspective];
                match app.hex_ui.interact_mode {
                    InteractMode::View => {
                        view.scroll_page_up();
                    }
                    InteractMode::Edit => {
                        #[expect(clippy::cast_sign_loss, reason = "view::rows is never negative")]
                        {
                            app.edit_state.cursor = app
                                .edit_state
                                .cursor
                                .saturating_sub(view.rows() as usize * per.cols);
                        }
                        keep_cursor_in_view(
                            view,
                            &app.meta_state.meta.low.perspectives,
                            &app.meta_state.meta.low.regions,
                            app.edit_state.cursor,
                        );
                    }
                }
            }
        }
        Key::PageDown => {
            if let Some(key) = app.hex_ui.focused_view {
                let view = &mut app.meta_state.meta.views[key].view;
                let per = &app.meta_state.meta.low.perspectives[view.perspective];
                match app.hex_ui.interact_mode {
                    InteractMode::View => {
                        app.meta_state.meta.views[key].view.scroll_page_down();
                    }
                    InteractMode::Edit => {
                        #[expect(clippy::cast_sign_loss, reason = "view::rows is never negative")]
                        {
                            app.edit_state.cursor = app
                                .edit_state
                                .cursor
                                .saturating_add(view.rows() as usize * per.cols);
                        }
                        keep_cursor_in_view(
                            view,
                            &app.meta_state.meta.low.perspectives,
                            &app.meta_state.meta.low.regions,
                            app.edit_state.cursor,
                        );
                    }
                }
            }
        }
        Key::Home => {
            if let Some(key) = app.hex_ui.focused_view {
                let view = &mut app.meta_state.meta.views[key].view;
                match app.hex_ui.interact_mode {
                    InteractMode::View => {
                        view.go_home();
                    }
                    InteractMode::Edit if key_mod.ctrl => {
                        view.go_home();
                        app.edit_state.cursor = app.meta_state.meta.low.start_offset_of_view(view);
                    }
                    InteractMode::Edit => {
                        if let Some(row_start) = app.find_row_start(app.edit_state.cursor) {
                            app.edit_state.cursor = row_start;
                            keep_cursor_in_view(
                                &mut app.meta_state.meta.views[key].view,
                                &app.meta_state.meta.low.perspectives,
                                &app.meta_state.meta.low.regions,
                                app.edit_state.cursor,
                            );
                        }
                    }
                }
            }
        }
        Key::End => {
            if let Some(key) = app.hex_ui.focused_view {
                let view = &mut app.meta_state.meta.views[key].view;
                match app.hex_ui.interact_mode {
                    InteractMode::View => {
                        app.meta_state.meta.views[key].view.scroll_to_end(
                            &app.meta_state.meta.low.perspectives,
                            &app.meta_state.meta.low.regions,
                        );
                    }
                    InteractMode::Edit if key_mod.ctrl => {
                        app.edit_state.cursor = app.meta_state.meta.low.end_offset_of_view(view);
                        app.center_view_on_offset(app.edit_state.cursor);
                    }
                    InteractMode::Edit => {
                        if let Some(row_end) = app.find_row_end(app.edit_state.cursor) {
                            app.edit_state.cursor = row_end;
                            keep_cursor_in_view(
                                &mut app.meta_state.meta.views[key].view,
                                &app.meta_state.meta.low.perspectives,
                                &app.meta_state.meta.low.regions,
                                app.edit_state.cursor,
                            );
                        }
                    }
                }
            }
        }
        Key::Delete => {
            if let Some(sel) = app.hex_ui.selection() {
                app.data.zero_fill_region(sel);
            } else if let Some(byte) = app.data.get_mut(app.edit_state.cursor) {
                *byte = 0;
            }
        }
        Key::F1 => app.hex_ui.interact_mode = InteractMode::View,
        Key::F2 => app.hex_ui.interact_mode = InteractMode::Edit,
        Key::F5 => gui.win.layouts.open.toggle(),
        Key::F6 => gui.win.views.open.toggle(),
        Key::F7 => gui.win.perspectives.open.toggle(),
        Key::F8 => gui.win.regions.open.toggle(),
        Key::F9 => gui.win.bookmarks.open.toggle(),
        Key::F10 => gui.win.vars.open.toggle(),
        Key::F11 => gui.win.structs.open.toggle(),
        Key::Escape => {
            gui.context_menu = None;
            if let Some(view_key) = app.hex_ui.focused_view {
                app.meta_state.meta.views[view_key].view.cancel_editing();
            }
            app.hex_ui.select_a = None;
            app.hex_ui.select_b = None;
        }
        Key::Enter => {
            if let Some(view_key) = app.hex_ui.focused_view {
                app.meta_state.meta.views[view_key].view.finish_editing(
                    &mut app.edit_state,
                    &mut app.data,
                    &app.preferences,
                    &mut gui.msg_dialog,
                );
            }
        }
        Key::A if key_mod.ctrl => {
            app.focused_view_select_all();
        }
        Key::E if key_mod.ctrl => {
            gui.win.external_command.open.set(true);
        }
        Key::F if key_mod.ctrl => {
            gui.win.find.open.toggle();
        }
        Key::S if key_mod.ctrl => match &mut app.source {
            Some(source) => {
                if !source.attr.permissions.write {
                    gui.msg_dialog.open(
                        Icon::Warn,
                        "Cannot save",
                        "This source cannot be written to.",
                    );
                } else {
                    msg_if_fail(
                        app.save(&mut gui.msg_dialog),
                        "Failed to save",
                        &mut gui.msg_dialog,
                    );
                }
            }
            None => gui.msg_dialog.open(Icon::Warn, "Cannot save", "No source opened"),
        },
        Key::R if key_mod.ctrl => {
            msg_if_fail(app.reload(), "Failed to reload", &mut gui.msg_dialog);
        }
        Key::P if key_mod.ctrl => {
            let mut load = None;
            crate::shell::open_previous(app, &mut load);
            if let Some(args) = load {
                msg_if_fail(
                    app.load_file_args(args, None, &mut gui.msg_dialog, font_size, line_spacing),
                    "Failed to load file",
                    &mut gui.msg_dialog,
                );
            }
        }
        Key::W if key_mod.ctrl => app.close_file(),
        Key::J if key_mod.ctrl => Gui::add_dialog(&mut gui.dialogs, JumpDialog::default()),
        Key::Num1 if key_mod.shift => app.hex_ui.select_a = Some(app.edit_state.cursor),
        Key::Num2 if key_mod.shift => app.hex_ui.select_b = Some(app.edit_state.cursor),
        Key::Tab if key_mod.shift => app.focus_prev_view_in_layout(),
        Key::Tab => app.focus_next_view_in_layout(),
        Key::Equal if key_mod.ctrl => app.inc_byte_at_cursor(),
        Key::Hyphen if key_mod.ctrl => app.dec_byte_at_cursor(),
        _ => {}
    }
}

fn keep_cursor_in_view(
    view: &mut view::View,
    perspectives: &PerspectiveMap,
    regions: &RegionMap,
    cursor: usize,
) {
    let view_offs = view.offsets(perspectives, regions);
    let (cur_row, cur_col) = perspectives[view.perspective].row_col_of_byte_offset(cursor, regions);
    view.scroll_offset.pix_xoff = 0;
    view.scroll_offset.pix_yoff = 0;
    if view_offs.row > cur_row {
        view.scroll_offset.row = cur_row;
    }
    #[expect(clippy::cast_sign_loss, reason = "rows is always unsigned")]
    let view_rows = view.rows() as usize;
    if (view_offs.row + view_rows) < cur_row.saturating_add(1) {
        view.scroll_offset.row = (cur_row + 1) - view_rows;
    }
    if view_offs.col > cur_col {
        view.scroll_offset.col = cur_col;
    }
    #[expect(clippy::cast_sign_loss, reason = "cols is always unsigned")]
    let view_cols = view.cols() as usize;
    if (view_offs.col + view_cols) < cur_col {
        view.scroll_offset.col = cur_col - view_cols;
    }
}
