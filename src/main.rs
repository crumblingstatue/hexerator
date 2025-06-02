#![doc(html_no_source)]
#![feature(
    try_blocks,
    array_chunks,
    array_windows,
    generic_const_exprs,
    macro_metavar_expr_concat,
    default_field_values,
    yeet_expr,
    cmp_minmax
)]
#![warn(
    unused_qualifications,
    redundant_imports,
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
    clippy::undocumented_unsafe_blocks,
    clippy::uninlined_format_args,
    clippy::format_push_string,
    clippy::unnecessary_wraps,
    clippy::map_unwrap_or,
    clippy::use_self,
    clippy::redundant_clone
)]
#![cfg_attr(test, allow(clippy::unwrap_used))]
#![expect(
    incomplete_features,
    // It's hard to reconcile lack of partial borrows with few arguments
    clippy::too_many_arguments
)]
#![windows_subsystem = "windows"]

use {
    crate::app::App,
    anyhow::Context as _,
    args::Args,
    clap::Parser as _,
    config::{Config, LoadedConfig, PinnedDir, ProjectDirsExt as _},
    constcat::concat,
    core::f32,
    egui_colors::{Colorix, tokens::ThemeColor},
    egui_file_dialog::PinnedFolder,
    egui_phosphor::regular as ic,
    egui_sf2g::{
        SfEgui,
        sf2g::{
            graphics::{Color, Font, RenderTarget as _, RenderWindow},
            system::Vector2,
            window::{ContextSettings, Event, Style, VideoMode},
        },
    },
    gui::{Gui, command::GCmd, message_dialog::Icon},
    mlua::Lua,
    std::{
        backtrace::{Backtrace, BacktraceStatus},
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
mod scripting;
mod session_prefs;
mod shell;
mod slice_ext;
mod source;
mod str_ext;
mod struct_meta_item;
mod timer;
mod update;
mod util;
mod value_color;
mod view;
#[cfg(windows)]
mod windows;

const L_CONTINUE: &str = concat!(ic::WARNING, " Continue");
const L_ABORT: &str = concat!(ic::X_CIRCLE, "Abort");

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
    // Show arg parse diagnostics in GUI window if stderr is not a terminal.
    //
    // This is the only way to get arg parse diagnostics on windows, due to windows_subsystem=windows
    let mut args = if std::io::stderr().is_terminal() {
        Args::parse()
    } else {
        match Args::try_parse() {
            Ok(args) => args,
            Err(e) => {
                do_fatal_error_report(
                    "Arg parse error",
                    &e.to_string(),
                    &Backtrace::force_capture(),
                );
                return Ok(());
            }
        }
    };
    if args.debug {
        gamedebug_core::IMMEDIATE.set_enabled(true);
        gamedebug_core::PERSISTENT.set_enabled(true);
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
        gui.msg_dialog.custom_button_row_ui(Box::new(|ui, payload, _cmd| {
            if ui.button(L_CONTINUE).clicked() {
                payload.close = true;
            }
            if ui.button(L_ABORT).clicked() {
                std::process::abort();
            }
        }));
    }
    let mut font_defs = egui::FontDefinitions::default();
    egui_phosphor::add_to_fonts(&mut font_defs, egui_phosphor::Variant::Regular);
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
    gui::set_font_sizes_style(&mut style, &app.cfg.style);
    sf_egui.context().set_style(style);
    // Custom egui_colors theme load
    if let Some(project_dirs) = config::project_dirs() {
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
        if !update::do_frame(
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
        if app.meta_state.last_meta_backup.get().elapsed() >= Duration::from_secs(60)
            && let Err(e) = app.save_temp_metafile_backup()
        {
            gamedebug_core::per!("Failed to save temp metafile backup: {}", e);
        }
    }
    app.close_file();
    transfer_pinned_folders_to_config(gui, &mut app);
    app.cfg.save()?;
    Ok(())
}

fn transfer_pinned_folders_to_file_dialog(gui: &mut Gui, cfg: &mut Config) {
    let dia_store = gui.fileops.dialog.storage_mut();
    // Remove them from the config, as later it will be filled with
    // the pinned dirs from the dialog
    for dir in cfg.pinned_dirs.drain(..) {
        dia_store.pinned_folders.push(PinnedFolder {
            label: dir.label,
            path: dir.path,
        });
    }
}

fn transfer_pinned_folders_to_config(mut gui: Gui, app: &mut App) {
    let storage = gui.fileops.dialog.storage_mut();
    for entry in std::mem::take(&mut storage.pinned_folders) {
        app.cfg.pinned_dirs.push(PinnedDir {
            path: entry.path,
            label: entry.label,
        });
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
