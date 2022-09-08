use egui_sfml::{egui, sfml::graphics::Font};

use crate::{app::App, args::Args, shell::msg_if_fail};

use super::{window_open::WindowOpen, Gui};

#[derive(Default)]
pub struct AdvancedOpenWindow {
    pub open: WindowOpen,
    pub args: Args,
}

fn opt<V: Default>(
    ui: &mut egui::Ui,
    val: &mut Option<V>,
    label: &str,
    desc: &str,
    f: impl FnOnce(&mut egui::Ui, &mut V),
) {
    ui.horizontal(|ui| {
        let mut checked = val.is_some();
        ui.checkbox(&mut checked, label).on_hover_text(desc);
        if checked {
            f(ui, val.get_or_insert_with(Default::default))
        } else {
            *val = None;
        }
    })
    .inner
}

impl AdvancedOpenWindow {
    pub fn ui(ui: &mut egui::Ui, gui: &mut Gui, app: &mut App, font: &Font) {
        let win = &mut gui.advanced_open_window;
        let args = &mut win.args;
        ui.heading("Source");
        match &args.src.file {
            Some(file) => {
                ui.label(file.display().to_string());
            }
            None => {
                ui.label("<No file selected>");
            }
        }
        if ui.button("Select file...").clicked() {
            if let Some(path) = rfd::FileDialog::new().pick_file() {
                args.src.file = Some(path);
            }
        }
        opt(
            ui,
            &mut args.src.jump,
            "jump",
            "Jump to offset on startup",
            |ui, jump| {
                ui.add(egui::DragValue::new(jump));
            },
        );
        opt(
            ui,
            &mut args.src.hard_seek,
            "hard seek",
            "Seek to offset, consider it beginning of the file in the editor",
            |ui, hard_seek| {
                ui.add(egui::DragValue::new(hard_seek));
            },
        );
        opt(
            ui,
            &mut args.src.take,
            "take",
            "Read only this many bytes",
            |ui, take| {
                ui.add(egui::DragValue::new(take));
            },
        );
        ui.checkbox(&mut args.src.read_only, "read-only")
            .on_hover_text("Open file as read-only");
        if ui
            .checkbox(&mut args.src.stream, "stream")
            .on_hover_text(
                "Specify source as a streaming source (for example, standard streams).\n\
             Sets read-only attribute",
            )
            .changed()
        {
            args.src.read_only = args.src.stream;
        }
        ui.heading("Meta");
        match &args.meta {
            Some(file) => {
                ui.label(file.display().to_string());
            }
            None => {
                ui.label("<No meta file selected>");
            }
        }
        if ui.button("Select file...").clicked() {
            if let Some(path) = rfd::FileDialog::new().pick_file() {
                args.meta = Some(path);
            }
        }
        ui.separator();
        if ui
            .add_enabled(args.src.file.is_some(), egui::Button::new("Load"))
            .clicked()
        {
            msg_if_fail(
                app.load_file_args(args.clone(), font),
                "Failed to load file",
            );
            win.open.set(false);
        }
    }
}
