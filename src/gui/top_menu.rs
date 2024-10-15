use {crate::shell::msg_if_fail, mlua::Lua};

mod analysis;
mod cursor;
pub mod edit;
mod file;
mod help;
mod meta;
mod perspective;
mod plugins;
mod scripting;
mod view;

use {
    crate::{app::App, source::SourceProvider},
    egui::Layout,
};

pub fn top_menu(
    ui: &mut egui::Ui,
    gui: &mut crate::gui::Gui,
    app: &mut App,
    lua: &Lua,
    font_size: u16,
    line_spacing: u16,
) {
    ui.horizontal(|ui| {
        ui.menu_button("File", |ui| file::ui(ui, gui, app, font_size, line_spacing));
        ui.menu_button("Edit", |ui| {
            edit::ui(ui, gui, app, lua, font_size, line_spacing)
        });
        ui.menu_button("Cursor", |ui| cursor::ui(ui, gui, app));
        ui.menu_button("View", |ui| view::ui(ui, gui, app));
        ui.menu_button("Perspective", |ui| perspective::ui(ui, gui, app));
        ui.menu_button("Meta", |ui| meta::ui(ui, gui, app, font_size, line_spacing));
        ui.menu_button("Analysis", |ui| analysis::ui(ui, gui, app));
        ui.menu_button("Scripting", |ui| {
            scripting::ui(ui, gui, app, lua, font_size, line_spacing)
        });
        ui.menu_button("Plugins", |ui| plugins::ui(ui, gui, app));
        ui.menu_button("Help", |ui| help::ui(ui, gui));
        ui.with_layout(
            Layout::right_to_left(egui::Align::Center),
            |ui| match &app.source {
                Some(src) => {
                    match src.provider {
                        SourceProvider::File(_) => {
                            match &app.src_args.file {
                                Some(file) => {
                                    let s = file.display().to_string();
                                    let ctx_menu = |ui: &mut egui::Ui| {
                                        if ui.button("Open").clicked() {
                                            try_open_file(file, gui);
                                            ui.close_menu();
                                        }
                                        if let Some(parent) = file.parent() {
                                            if ui.button("Open containing folder").clicked() {
                                                let result = open::that(parent);
                                                msg_if_fail(
                                                    result,
                                                    "Failed to open folder",
                                                    &mut gui.msg_dialog,
                                                );
                                                ui.close_menu();
                                            }
                                        }
                                        if ui.button("Copy path to clipboard").clicked() {
                                            crate::app::set_clipboard_string(
                                                &mut app.clipboard,
                                                &mut gui.msg_dialog,
                                                &s,
                                            );
                                            ui.close_menu();
                                        }
                                    };
                                    if !app.meta_state.current_meta_path.as_os_str().is_empty() {
                                        ui.label(
                                            egui::RichText::new("🇲").color(egui::Color32::YELLOW),
                                        )
                                        .on_hover_text(
                                            format!(
                                                "Metafile: {}",
                                                app.meta_state.current_meta_path.display()
                                            ),
                                        );
                                    } else {
                                        ui.label("？").on_hover_text(
                                            "There is no metafile associated with this file",
                                        );
                                    }
                                    let mut re =
                                        ui.add(egui::Label::new(&s).sense(egui::Sense::click()));
                                    re.context_menu(ctx_menu);
                                    re = re.on_hover_ui(|ui| {
                                        if let Some(offset) = &app.src_args.hard_seek {
                                            ui.label(format!("Hard seek: {offset} ({offset:X})"));
                                        }
                                        if let Some(len) = &app.src_args.take {
                                            ui.label(format!("Take: {len}"));
                                        }
                                        ui.label("Right click for context menu");
                                    });
                                    if re.clicked() {
                                        try_open_file(file, gui);
                                    }
                                }
                                None => {
                                    ui.label("File path unknown");
                                }
                            };
                        }
                        SourceProvider::Stdin(_) => {
                            ui.label("Standard input");
                        }
                        #[cfg(windows)]
                        SourceProvider::WinProc { handle, .. } => {
                            ui.label(format!("Windows process: {:p}", handle));
                        }
                    }
                    if src.attr.stream {
                        if src.state.stream_end {
                            ui.label("[finished stream]");
                        } else {
                            ui.spinner();
                            ui.label("[streaming]");
                        }
                    }
                }
                None => {
                    ui.label("No source");
                }
            },
        );
    });
}

fn try_open_file(file: &std::path::Path, gui: &mut super::Gui) {
    let result = open::that(file);
    msg_if_fail(result, "Failed to open file", &mut gui.msg_dialog);
}
