use {
    crate::{
        app::App,
        gui::{util::button_with_shortcut, Gui},
        shell::msg_if_fail,
    },
    egui,
};

pub fn ui(ui: &mut egui::Ui, gui: &mut Gui, app: &mut App) {
    if button_with_shortcut(ui, "Regions...", "F8").clicked() {
        gui.regions_window.open.toggle();
        ui.close_menu();
    }
    if button_with_shortcut(ui, "Bookmarks...", "F9").clicked() {
        gui.bookmarks_window.open.toggle();
        ui.close_menu();
    }
    ui.separator();
    if ui
        .button("Diff with clean meta")
        .on_hover_text("See and manage changes to metafile")
        .clicked()
    {
        gui.meta_diff_window.open.toggle();
        ui.close_menu();
    }
    ui.separator();
    if ui
        .add_enabled(
            !app.meta_state.current_meta_path.as_os_str().is_empty(),
            egui::Button::new("Reload"),
        )
        .on_hover_text(format!(
            "Reload from {}",
            app.meta_state.current_meta_path.display()
        ))
        .clicked()
    {
        msg_if_fail(
            app.consume_meta_from_file(app.meta_state.current_meta_path.clone()),
            "Failed to load metafile",
            &mut gui.msg_dialog,
        );
        ui.close_menu();
    }
    if ui.button("Load from file...").clicked() {
        if let Some(path) = rfd::FileDialog::default().pick_file() {
            msg_if_fail(
                app.consume_meta_from_file(path),
                "Failed to load metafile",
                &mut gui.msg_dialog,
            );
        }
        ui.close_menu();
    }
    if ui
        .button("Load from temp backup")
        .on_hover_text("Load from temporary backup (auto generated on save/exit)")
        .clicked()
    {
        msg_if_fail(
            app.consume_meta_from_file(crate::app::temp_metafile_backup_path()),
            "Failed to load temp metafile",
            &mut gui.msg_dialog,
        );
        ui.close_menu();
    }
    ui.separator();
    if ui
        .add_enabled(
            !app.meta_state.current_meta_path.as_os_str().is_empty(),
            egui::Button::new("Save"),
        )
        .on_hover_text(format!(
            "Save to {}",
            app.meta_state.current_meta_path.display()
        ))
        .clicked()
    {
        msg_if_fail(
            app.save_meta_to_file(app.meta_state.current_meta_path.clone(), false),
            "Failed to save metafile",
            &mut gui.msg_dialog,
        );
        ui.close_menu();
    }
    if ui.button("Save as...").clicked() {
        if let Some(path) = rfd::FileDialog::default().save_file() {
            msg_if_fail(
                app.save_meta_to_file(path, false),
                "Failed to save metafile",
                &mut gui.msg_dialog,
            );
        }
        ui.close_menu();
    }
}
