use {
    crate::{
        app::{set_clipboard_string, App},
        gui::{dialogs::AutoSaveReloadDialog, Gui},
        shell::msg_if_fail,
    },
    egui::Button,
};

pub fn ui(ui: &mut egui::Ui, gui: &mut Gui, app: &mut App, font_size: u16, line_spacing: u16) {
    if ui.add(Button::new("Open...").shortcut_text("Ctrl+O")).clicked() {
        gui.fileops.load_file(app.source_file());
        ui.close_menu();
    }
    if ui.button("Advanced open...").clicked() {
        gui.win.advanced_open.open.toggle();
        ui.close_menu();
    }
    if ui.button("Open process...").clicked() {
        gui.win.open_process.open.toggle();
        ui.close_menu();
    }
    let mut load = None;
    if ui
        .add_enabled(
            !app.cfg.recent.is_empty(),
            Button::new("Open previous").shortcut_text("Ctrl+P"),
        )
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
        ui.horizontal(|ui| {
            let mut cap = app.cfg.recent.capacity();
            if ui.add(egui::DragValue::new(&mut cap).prefix("list capacity: ")).changed() {
                app.cfg.recent.set_capacity(cap);
            }
            ui.separator();
            if ui
                .add_enabled(!app.cfg.recent.is_empty(), egui::Button::new("🗑 Clear all"))
                .clicked()
            {
                app.cfg.recent.clear();
            }
        });
    });
    if let Some(args) = load {
        msg_if_fail(
            app.load_file_args(args, None, &mut gui.msg_dialog, font_size, line_spacing),
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
        msg_if_fail(
            app.save(&mut gui.msg_dialog),
            "Failed to save",
            &mut gui.msg_dialog,
        );
        ui.close_menu();
    }
    if ui.button("Save as...").clicked() {
        gui.fileops.save_file_as();
        ui.close_menu();
    }
    if ui.add(Button::new("Reload").shortcut_text("Ctrl+R")).clicked() {
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
        gui.win.preferences.open.toggle();
        ui.close_menu();
    }
    ui.separator();
    if ui.add(Button::new("Close").shortcut_text("Ctrl+W")).clicked() {
        app.close_file();
        ui.close_menu();
    }
    if ui.button("Quit").clicked() {
        app.quit_requested = true;
    }
}
