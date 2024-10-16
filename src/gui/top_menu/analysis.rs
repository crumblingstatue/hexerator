use crate::{
    app::App,
    gui::{message_dialog::Icon, Gui},
    shell::msg_if_fail,
};

pub fn ui(ui: &mut egui::Ui, gui: &mut Gui, app: &App) {
    if ui.button("Determine data mime type under cursor").clicked() {
        gui.msg_dialog.open(
            Icon::Info,
            "Data mime type under cursor",
            tree_magic_mini::from_u8(&app.data[app.edit_state.cursor..]).to_string(),
        );
        ui.close_menu();
    }
    if let Some(region) = app.hex_ui.selection() {
        if ui.button("Determine data mime type of selection").clicked() {
            gui.msg_dialog.open(
                Icon::Info,
                "Data mime type under cursor",
                tree_magic_mini::from_u8(&app.data[region.begin..=region.end]).to_string(),
            );
            ui.close_menu();
        }
    }
    ui.separator();
    if ui.button("Diff with file...").clicked() {
        gui.fileops.diff_with_file(app.source_file());
        ui.close_menu();
    }
    if ui.button("Diff with source file").clicked() {
        ui.close_menu();
        if let Some(path) = app.source_file() {
            let path = path.to_owned();
            msg_if_fail(
                app.diff_with_file(path, &mut gui.win.file_diff_result),
                "Failed to diff",
                &mut gui.msg_dialog,
            );
        }
    }
    match app.backup_path() {
        Some(path) if path.exists() => {
            if ui.button("Diff with backup").clicked() {
                ui.close_menu();
                msg_if_fail(
                    app.diff_with_file(path, &mut gui.win.file_diff_result),
                    "Failed to diff",
                    &mut gui.msg_dialog,
                );
            }
        }
        _ => {
            ui.add_enabled(false, egui::Button::new("Diff with backup"));
        }
    }
    ui.separator();
    if ui
        .add_enabled(
            gui.win.open_process.selected_pid.is_some(),
            egui::Button::new("Find memory pointers..."),
        )
        .clicked()
    {
        gui.win.find_memory_pointers.open.toggle();
        ui.close_menu()
    }
    if ui
        .button("Zero partition...")
        .on_hover_text("Find regions of non-zero data separated by zeroed regions")
        .clicked()
    {
        gui.win.zero_partition.open.toggle();
        ui.close_menu();
    }
}
