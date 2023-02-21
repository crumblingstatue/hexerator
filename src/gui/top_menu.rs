use crate::{event::EventQueue, shell::msg_if_fail};

mod analysis;
mod cursor;
pub mod edit;
mod file;
mod help;
mod meta;
mod perspective;
mod scripting;
mod view;

use {
    crate::{app::App, source::SourceProvider},
    egui_sfml::{
        egui::{self, Layout},
        sfml::graphics::Font,
    },
};

pub fn top_menu(
    ui: &mut egui::Ui,
    gui: &mut crate::gui::Gui,
    app: &mut App,
    font: &Font,
    events: &mut EventQueue,
) {
    ui.horizontal(|ui| {
        ui.menu_button("File", |ui| file::ui(ui, gui, app, font, events));
        ui.menu_button("Edit", |ui| edit::ui(ui, gui, app, events));
        ui.menu_button("Cursor", |ui| cursor::ui(ui, gui, app));
        ui.menu_button("View", |ui| view::ui(ui, gui, app));
        ui.menu_button("Perspective", |ui| perspective::ui(ui, gui, app));
        ui.menu_button("Meta", |ui| meta::ui(ui, gui, app, font));
        ui.menu_button("Analysis", |ui| analysis::ui(ui, gui, app));
        ui.menu_button("Scripting", |ui| scripting::ui(ui, gui, app));
        ui.menu_button("Help", |ui| help::ui(ui, gui));
        ui.with_layout(
            Layout::right_to_left(egui::Align::Center),
            |ui| match &app.source {
                Some(src) => {
                    match src.provider {
                        SourceProvider::File(_) => {
                            match &app.args.src.file {
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
                                    if ui
                                        .add(egui::Label::new(&s).sense(egui::Sense::click()))
                                        .context_menu(ctx_menu)
                                        .on_hover_text("Right click for context menu")
                                        .clicked()
                                    {
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
                            ui.label(format!("Windows process: {}", handle));
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
