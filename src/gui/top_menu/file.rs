use {
    crate::{
        app::App,
        args::Args,
        gui::{
            dialogs::AutoSaveReloadDialog,
            util::{button_with_shortcut, ButtonWithShortcut},
            Gui,
        },
        shell::msg_if_fail,
    },
    egui_sfml::{egui, sfml::graphics::Font},
};

pub fn ui(ui: &mut egui::Ui, gui: &mut Gui, app: &mut App, font: &Font) {
    if button_with_shortcut(ui, "Open...", "Ctrl+O").clicked() {
        crate::shell::open_file(app, font, &mut gui.msg_dialog);
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
    if button_with_shortcut(ui, "Open previous", "Ctrl+P")
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
            ui.horizontal(|ui| {
                if ui
                    .button(
                        entry
                            .file
                            .as_ref()
                            .map(|path| path.display().to_string())
                            .unwrap_or_else(|| String::from("Unnamed file")),
                    )
                    .clicked()
                {
                    load = Some(entry.clone());
                    ui.close_menu();
                }
                ui.separator();
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
            ),
            "Failed to load file",
            &mut gui.msg_dialog,
        );
    }
    ui.separator();
    if ui
        .add_enabled(
            app.source
                .as_ref()
                .is_some_and(|src| src.attr.permissions.write)
                && app.edit_state.dirty_region.is_some(),
            ButtonWithShortcut("Save", "Ctrl+S"),
        )
        .clicked()
    {
        msg_if_fail(app.save(), "Failed to save", &mut gui.msg_dialog);
        ui.close_menu();
    }
    if button_with_shortcut(ui, "Reload", "Ctrl+R").clicked() {
        msg_if_fail(app.reload(), "Failed to reload", &mut gui.msg_dialog);
        ui.close_menu();
    }
    if ui.button("Auto save/reload...").clicked() {
        ui.close_menu();
        gui.add_dialog(AutoSaveReloadDialog);
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
    if button_with_shortcut(ui, "Close", "Ctrl+W").clicked() {
        app.close_file();
        ui.close_menu();
    }
}
