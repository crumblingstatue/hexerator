mod analysis;
mod cursor;
mod edit;
mod file;
mod help;
mod meta;
mod perspective;
mod view;

use {
    crate::{app::App, source::SourceProvider},
    egui_sfml::{
        egui::{self, Layout},
        sfml::graphics::Font,
    },
};

pub fn top_menu(ui: &mut egui::Ui, gui: &mut crate::gui::Gui, app: &mut App, font: &Font) {
    ui.horizontal(|ui| {
        ui.menu_button("File", |ui| file::ui(ui, gui, app, font));
        ui.menu_button("Edit", |ui| edit::ui(ui, gui, app));
        ui.menu_button("Cursor", |ui| cursor::ui(ui, gui, app));
        ui.menu_button("View", |ui| view::ui(ui, gui, app));
        ui.menu_button("Perspective", |ui| perspective::ui(ui, gui, app));
        ui.menu_button("Meta", |ui| meta::ui(ui, gui, app));
        ui.menu_button("Analysis", |ui| analysis::ui(ui, gui, app));
        ui.menu_button("Help", |ui| help::ui(ui, gui));
        ui.with_layout(
            Layout::right_to_left(egui::Align::Center),
            |ui| match &app.source {
                Some(src) => {
                    match src.provider {
                        SourceProvider::File(_) => {
                            match &app.args.src.file {
                                Some(file) => ui.label(file.display().to_string()),
                                None => ui.label("File path unknown"),
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
                    ui.label("No source loaded");
                }
            },
        );
    });
}
