use {
    crate::{
        app::App,
        gui::{Gui, egui_ui_ext::EguiResponseExt as _},
        shell::msg_if_fail,
    },
    egui::Button,
};

pub fn ui(ui: &mut egui::Ui, gui: &mut Gui, app: &mut App, font_size: u16, line_spacing: u16) {
    if ui.add(Button::new("Regions...").shortcut_text("F8")).clicked() {
        gui.win.regions.open.toggle();
        ui.close_menu();
    }
    if ui.add(Button::new("Bookmarks...").shortcut_text("F9")).clicked() {
        gui.win.bookmarks.open.toggle();
        ui.close_menu();
    }
    if ui.add(Button::new("Variables").shortcut_text("F10")).clicked() {
        gui.win.vars.open.toggle();
        ui.close_menu();
    }
    if ui.add(Button::new("Structs").shortcut_text("F11")).clicked() {
        gui.win.structs.open.toggle();
        ui.close_menu();
    }
    ui.separator();
    if ui
        .button("Diff with clean meta")
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
            Button::new("Reload"),
        )
        .on_hover_text_deferred(|| {
            format!("Reload from {}", app.meta_state.current_meta_path.display())
        })
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
        gui.fileops.load_meta_file();
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
    if ui
        .button("🗑 Clear")
        .on_hover_text("Replace current meta with default one")
        .clicked()
    {
        app.set_new_clean_meta(font_size, line_spacing);
        ui.close_menu();
    }
    ui.separator();
    if ui
        .add_enabled(
            !app.meta_state.current_meta_path.as_os_str().is_empty(),
            Button::new("Save"),
        )
        .on_hover_text_deferred(|| {
            format!("Save to {}", app.meta_state.current_meta_path.display())
        })
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
                .button("Associate with current file")
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
            ui.add_enabled(false, Button::new("Associate with current file"))
                .on_disabled_hover_text("Both file and metafile need to have a path");
        }
    }
}
