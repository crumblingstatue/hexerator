use {
    crate::{
        app::App,
        gui::{Gui, message_dialog::Icon},
        shell::msg_if_fail,
    },
    constcat::concat,
    egui_phosphor::regular as ic,
};

const L_DETERMINE_DATA_MIME: &str =
    concat!(ic::SEAL_QUESTION, " Determine data mime type under cursor");
const L_DETERMINE_DATA_MIME_SEL: &str =
    concat!(ic::SEAL_QUESTION, " Determine data mime type of selection");
const L_DIFF_WITH_FILE: &str = concat!(ic::GIT_DIFF, " Diff with file...");
const L_DIFF_WITH_SOURCE_FILE: &str = concat!(ic::GIT_DIFF, " Diff with source file");
const L_DIFF_WITH_BACKUP: &str = concat!(ic::GIT_DIFF, " Diff with backup");
const L_FIND_MEMORY_POINTERS: &str = concat!(ic::ARROW_UP_RIGHT, " Find memory pointers...");
const L_ZERO_PARTITION: &str = concat!(ic::BINARY, " Zero partition...");

pub fn ui(ui: &mut egui::Ui, gui: &mut Gui, app: &App) {
    if ui.button(L_DETERMINE_DATA_MIME).clicked() {
        gui.msg_dialog.open(
            Icon::Info,
            "Data mime type under cursor",
            tree_magic_mini::from_u8(&app.data[app.edit_state.cursor..]).to_string(),
        );
    }
    if let Some(region) = app.hex_ui.selection()
        && ui.button(L_DETERMINE_DATA_MIME_SEL).clicked()
    {
        gui.msg_dialog.open(
            Icon::Info,
            "Data mime type of selection",
            tree_magic_mini::from_u8(&app.data[region.begin..=region.end]).to_string(),
        );
    }
    ui.separator();
    if ui.button(L_DIFF_WITH_FILE).clicked() {
        gui.fileops.diff_with_file(app.source_file());
    }
    if ui.button(L_DIFF_WITH_SOURCE_FILE).clicked()
        && let Some(path) = app.source_file()
    {
        let path = path.to_owned();
        msg_if_fail(
            app.diff_with_file(path, &mut gui.win.file_diff_result),
            "Failed to diff",
            &mut gui.msg_dialog,
        );
    }
    match app.backup_path() {
        Some(path) if path.exists() => {
            if ui.button(L_DIFF_WITH_BACKUP).clicked() {
                msg_if_fail(
                    app.diff_with_file(path, &mut gui.win.file_diff_result),
                    "Failed to diff",
                    &mut gui.msg_dialog,
                );
            }
        }
        _ => {
            ui.add_enabled(false, egui::Button::new(L_DIFF_WITH_BACKUP));
        }
    }
    ui.separator();
    if ui
        .add_enabled(
            gui.win.open_process.selected_pid.is_some(),
            egui::Button::new(L_FIND_MEMORY_POINTERS),
        )
        .on_disabled_hover_text("Requires open process")
        .clicked()
    {
        gui.win.find_memory_pointers.open.toggle();
    }
    if ui
        .button(L_ZERO_PARTITION)
        .on_hover_text("Find regions of non-zero data separated by zeroed regions")
        .clicked()
    {
        gui.win.zero_partition.open.toggle();
    }
}
