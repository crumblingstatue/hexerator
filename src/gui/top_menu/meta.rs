use {
    crate::{
        app::App,
        gui::{Gui, egui_ui_ext::EguiResponseExt as _},
        shell::msg_if_fail,
    },
    constcat::concat,
    egui::Button,
    egui_phosphor::regular as ic,
};

const L_PERSPECTIVES: &str = concat!(ic::PERSPECTIVE, " Perspectives...");
const L_REGIONS: &str = concat!(ic::RULER, " Regions...");
const L_BOOKMARKS: &str = concat!(ic::BOOKMARK, " Bookmarks...");
const L_VARIABLES: &str = concat!(ic::CALCULATOR, " Variables...");
const L_STRUCTS: &str = concat!(ic::BLUEPRINT, " Structs...");
const L_RELOAD: &str = concat!(ic::ARROW_COUNTER_CLOCKWISE, " Reload");
const L_LOAD_FROM_FILE: &str = concat!(ic::FOLDER_OPEN, " Load from file...");
const L_LOAD_FROM_BACKUP: &str = concat!(ic::CLOUD_ARROW_DOWN, " Load from temp backup");
const L_CLEAR: &str = concat!(ic::BROOM, " Clear");
const L_DIFF_WITH_CLEAN_META: &str = concat!(ic::GIT_DIFF, " Diff with clean meta");
const L_SAVE: &str = concat!(ic::FLOPPY_DISK, " Save");
const L_SAVE_AS: &str = concat!(ic::FLOPPY_DISK_BACK, " Save as...");
const L_ASSOCIATE_WITH_CURRENT: &str = concat!(ic::FLOW_ARROW, " Associate with current file");

pub fn ui(ui: &mut egui::Ui, gui: &mut Gui, app: &mut App, font_size: u16, line_spacing: u16) {
    if ui.add(Button::new(L_PERSPECTIVES).shortcut_text("F7")).clicked() {
        gui.win.perspectives.open.toggle();
        ui.close_menu();
    }
    if ui.add(Button::new(L_REGIONS).shortcut_text("F8")).clicked() {
        gui.win.regions.open.toggle();
        ui.close_menu();
    }
    if ui.add(Button::new(L_BOOKMARKS).shortcut_text("F9")).clicked() {
        gui.win.bookmarks.open.toggle();
        ui.close_menu();
    }
    if ui.add(Button::new(L_VARIABLES).shortcut_text("F10")).clicked() {
        gui.win.vars.open.toggle();
        ui.close_menu();
    }
    if ui.add(Button::new(L_STRUCTS).shortcut_text("F11")).clicked() {
        gui.win.structs.open.toggle();
        ui.close_menu();
    }
    ui.separator();
    if ui
        .button(L_DIFF_WITH_CLEAN_META)
        .on_hover_text("See and manage changes to metafile")
        .clicked()
    {
        gui.win.meta_diff.open.toggle();
        ui.close_menu();
    }
    ui.separator();
    if ui
        .add_enabled(
            !app.meta_state.current_meta_path.as_os_str().is_empty(),
            Button::new(L_RELOAD),
        )
        .on_hover_text_deferred(|| {
            format!("Reload from {}", app.meta_state.current_meta_path.display())
        })
        .clicked()
    {
        msg_if_fail(
            app.consume_meta_from_file(app.meta_state.current_meta_path.clone(), false),
            "Failed to load metafile",
            &mut gui.msg_dialog,
        );
        ui.close_menu();
    }
    if ui.button(L_LOAD_FROM_FILE).clicked() {
        gui.fileops.load_meta_file();
        ui.close_menu();
    }
    if ui
        .button(L_LOAD_FROM_BACKUP)
        .on_hover_text("Load from temporary backup (auto generated on save/exit)")
        .clicked()
    {
        msg_if_fail(
            app.consume_meta_from_file(crate::app::temp_metafile_backup_path(), true),
            "Failed to load temp metafile",
            &mut gui.msg_dialog,
        );
        ui.close_menu();
    }
    if ui
        .button(L_CLEAR)
        .on_hover_text("Replace current meta with default one")
        .clicked()
    {
        app.clear_meta(font_size, line_spacing);
        ui.close_menu();
    }
    ui.separator();
    if ui
        .add_enabled(
            !app.meta_state.current_meta_path.as_os_str().is_empty(),
            Button::new(L_SAVE).shortcut_text("Ctrl+M"),
        )
        .on_hover_text_deferred(|| {
            format!("Save to {}", app.meta_state.current_meta_path.display())
        })
        .clicked()
    {
        msg_if_fail(
            app.save_meta(),
            "Failed to save metafile",
            &mut gui.msg_dialog,
        );
        ui.close_menu();
    }
    if ui.button(L_SAVE_AS).clicked() {
        gui.fileops.save_metafile_as();
        ui.close_menu();
    }
    ui.separator();
    match (
        app.source_file(),
        app.meta_state.current_meta_path.as_os_str().is_empty(),
    ) {
        (Some(src), false) => {
            if ui
                .button(L_ASSOCIATE_WITH_CURRENT)
                .on_hover_text("When you open this file, it will use this metafile")
                .clicked()
            {
                app.cfg
                    .meta_assocs
                    .insert(src.to_owned(), app.meta_state.current_meta_path.clone());
                ui.close_menu();
            }
        }
        _ => {
            ui.add_enabled(false, Button::new(L_ASSOCIATE_WITH_CURRENT))
                .on_disabled_hover_text("Both file and metafile need to have a path");
        }
    }
}
