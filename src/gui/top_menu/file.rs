use {
    crate::{
        app::{set_clipboard_string, App},
        args::{Args, SourceArgs},
        event::EventQueue,
        gui::{dialogs::AutoSaveReloadDialog, Gui},
        shell::msg_if_fail,
        source::{Source, SourceAttributes, SourcePermissions, SourceProvider, SourceState},
    },
    egui::Button,
    egui_sfml::{egui, sfml::graphics::Font},
    std::io::Write,
};

pub fn ui(ui: &mut egui::Ui, gui: &mut Gui, app: &mut App, font: &Font, events: &mut EventQueue) {
    if ui
        .add(Button::new("Open...").shortcut_text("Ctrl+O"))
        .clicked()
    {
        crate::shell::open_file(app, font, &mut gui.msg_dialog, events);
        ui.close_menu();
    }
    if ui.button("Advanced open...").clicked() {
        gui.advanced_open_window.open.toggle();
        ui.close_menu();
    }
    if ui.button("Open process...").clicked() {
        gui.open_process_window.open.toggle();
        ui.close_menu();
    }
    let mut load = None;
    if ui
        .add(Button::new("Open previous").shortcut_text("Ctrl+P"))
        .on_hover_text("Can be used to switch between 2 files quickly for comparison")
        .clicked()
    {
        crate::shell::open_previous(app, &mut load);
        ui.close_menu();
    }
    ui.checkbox(&mut app.preferences.keep_meta, "Keep metadata")
        .on_hover_text("Keep metadata when loading a new file");
    ui.menu_button("Recent", |ui| {
        app.cfg.recent.retain(|entry| {
            let mut retain = true;
            let path = entry
                .file
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| String::from("Unnamed file"));
            ui.horizontal(|ui| {
                if ui.button(&path).clicked() {
                    load = Some(entry.clone());
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("📋").clicked() {
                    set_clipboard_string(&mut app.clipboard, &mut gui.msg_dialog, &path);
                }
                if ui.button("🗑").clicked() {
                    retain = false;
                }
            });
            ui.separator();
            retain
        });
        ui.separator();
        let mut cap = app.cfg.recent.capacity();
        if ui
            .add(egui::DragValue::new(&mut cap).prefix("list capacity: "))
            .changed()
        {
            app.cfg.recent.set_capacity(cap);
        }
    });
    if let Some(args) = load {
        msg_if_fail(
            app.load_file_args(
                Args {
                    src: args,
                    recent: false,
                    meta: None,
                },
                font,
                &mut gui.msg_dialog,
                events,
            ),
            "Failed to load file",
            &mut gui.msg_dialog,
        );
    }
    ui.separator();
    if ui
        .add_enabled(
            matches!(&app.source, Some(src) if src.attr.permissions.write)
                && app.edit_state.dirty_region.is_some(),
            Button::new("Save").shortcut_text("Ctrl+S"),
        )
        .clicked()
    {
        msg_if_fail(app.save(), "Failed to save", &mut gui.msg_dialog);
        ui.close_menu();
    }
    if ui.button("Save as...").clicked() {
        if let Some(path) = rfd::FileDialog::new().save_file() {
            let result: anyhow::Result<()> = try {
                let mut f = std::fs::OpenOptions::new()
                    .create(true)
                    .truncate(true)
                    .read(true)
                    .write(true)
                    .open(&path)?;
                f.write_all(&app.data)?;
                app.source = Some(Source {
                    provider: SourceProvider::File(f),
                    attr: SourceAttributes {
                        seekable: true,
                        stream: false,
                        permissions: SourcePermissions {
                            read: true,
                            write: true,
                        },
                    },
                    state: SourceState::default(),
                });
                app.args.src.file = Some(path);
                app.cfg.recent.use_(SourceArgs {
                    file: app.args.src.file.clone(),
                    jump: None,
                    hard_seek: None,
                    take: None,
                    read_only: false,
                    stream: false,
                });
            };
            msg_if_fail(result, "Failed to save as", &mut gui.msg_dialog);
        }
    }
    if ui
        .add(Button::new("Reload").shortcut_text("Ctrl+R"))
        .clicked()
    {
        msg_if_fail(app.reload(), "Failed to reload", &mut gui.msg_dialog);
        ui.close_menu();
    }
    if ui.button("Auto save/reload...").clicked() {
        ui.close_menu();
        Gui::add_dialog(&mut gui.dialogs, AutoSaveReloadDialog);
    }
    ui.separator();
    if ui.button("Create backup").clicked() {
        msg_if_fail(
            app.create_backup(),
            "Failed to create backup",
            &mut gui.msg_dialog,
        );
        ui.close_menu();
    }
    if ui.button("Restore backup").clicked() {
        msg_if_fail(
            app.restore_backup(),
            "Failed to restore backup",
            &mut gui.msg_dialog,
        );
        ui.close_menu();
    }
    ui.separator();
    if ui.button("Preferences").clicked() {
        gui.preferences_window.open.toggle();
        ui.close_menu();
    }
    ui.separator();
    if ui
        .add(Button::new("Close").shortcut_text("Ctrl+W"))
        .clicked()
    {
        app.close_file();
        ui.close_menu();
    }
}
