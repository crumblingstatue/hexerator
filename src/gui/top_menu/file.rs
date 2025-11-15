use {
    crate::{
        app::{App, set_clipboard_string},
        gui::{Gui, dialogs::AutoSaveReloadDialog},
        shell::msg_if_fail,
    },
    constcat::concat,
    egui::Button,
    egui_phosphor::regular as ic,
};

const L_LOPEN: &str = concat!(ic::FOLDER_OPEN, " Open...");
const L_OPEN_PROCESS: &str = concat!(ic::CPU, " Open process...");
const L_OPEN_PREVIOUS: &str = concat!(ic::ARROWS_LEFT_RIGHT, " Open previous");
const L_SAVE: &str = concat!(ic::FLOPPY_DISK, " Save");
const L_SAVE_AS: &str = concat!(ic::FLOPPY_DISK_BACK, " Save as...");
const L_RELOAD: &str = concat!(ic::ARROW_COUNTER_CLOCKWISE, " Reload");
const L_RECENT: &str = concat!(ic::CLOCK_COUNTER_CLOCKWISE, " Recent");
const L_AUTO_SAVE_RELOAD: &str = concat!(ic::MAGNET, " Auto save/reload...");
const L_CREATE_BACKUP: &str = concat!(ic::CLOUD_ARROW_UP, " Create backup");
const L_RESTORE_BACKUP: &str = concat!(ic::CLOUD_ARROW_DOWN, " Restore backup");
const L_PREFERENCES: &str = concat!(ic::GEAR_SIX, " Preferences");
const L_CLOSE: &str = concat!(ic::X_SQUARE, " Close");
const L_QUIT: &str = concat!(ic::SIGN_OUT, " Quit");

pub fn ui(ui: &mut egui::Ui, gui: &mut Gui, app: &mut App, font_size: u16, line_spacing: u16) {
    if ui.add(Button::new(L_LOPEN).shortcut_text("Ctrl+O")).clicked() {
        gui.fileops.load_file(app.source_file());
    }
    if ui.button(L_OPEN_PROCESS).clicked() {
        gui.win.open_process.open.toggle();
    }
    let mut load = None;
    if ui
        .add_enabled(
            !app.cfg.recent.is_empty(),
            Button::new(L_OPEN_PREVIOUS).shortcut_text("Ctrl+P"),
        )
        .on_hover_text("Can be used to switch between 2 files quickly for comparison")
        .clicked()
    {
        crate::shell::open_previous(app, &mut load);
    }
    ui.checkbox(&mut app.preferences.keep_meta, "Keep metadata")
        .on_hover_text("Keep metadata when loading a new file");
    ui.menu_button(L_RECENT, |ui| {
        app.cfg.recent.retain(|entry| {
            let mut retain = true;
            let path = entry.file.as_ref().map_or_else(
                || String::from("Unnamed file"),
                |path| path.display().to_string(),
            );
            ui.horizontal(|ui| {
                if ui.button(&path).clicked() {
                    load = Some(entry.clone());
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
            if ui.add_enabled(!app.cfg.recent.is_empty(), Button::new("🗑 Clear all")).clicked() {
                app.cfg.recent.clear();
            }
        });
    });
    if let Some(args) = load {
        app.load_file_args(
            args,
            None,
            &mut gui.msg_dialog,
            font_size,
            line_spacing,
            None,
        );
    }
    ui.separator();
    if ui
        .add_enabled(
            matches!(&app.source, Some(src) if src.attr.permissions.write)
                && app.data.dirty_region.is_some(),
            Button::new(L_SAVE).shortcut_text("Ctrl+S"),
        )
        .clicked()
    {
        msg_if_fail(
            app.save(&mut gui.msg_dialog),
            "Failed to save",
            &mut gui.msg_dialog,
        );
    }
    if ui.button(L_SAVE_AS).clicked() {
        gui.fileops.save_file_as();
    }
    if ui.add(Button::new(L_RELOAD).shortcut_text("Ctrl+R")).clicked() {
        msg_if_fail(app.reload(), "Failed to reload", &mut gui.msg_dialog);
    }
    if ui.button(L_AUTO_SAVE_RELOAD).clicked() {
        Gui::add_dialog(&mut gui.dialogs, AutoSaveReloadDialog);
    }
    ui.separator();
    if ui.button(L_CREATE_BACKUP).clicked() {
        msg_if_fail(
            app.create_backup(),
            "Failed to create backup",
            &mut gui.msg_dialog,
        );
    }
    if ui.button(L_RESTORE_BACKUP).clicked() {
        msg_if_fail(
            app.restore_backup(),
            "Failed to restore backup",
            &mut gui.msg_dialog,
        );
    }
    ui.separator();
    if ui.button(L_PREFERENCES).clicked() {
        gui.win.preferences.open.toggle();
    }
    ui.separator();
    if ui.add(Button::new(L_CLOSE).shortcut_text("Ctrl+W")).clicked() {
        app.close_file();
    }
    if ui.button(L_QUIT).clicked() {
        app.quit_requested = true;
    }
}
