use {
    super::{WinCtx, WindowOpen},
    crate::{args::SourceArgs, shell::msg_if_fail},
    std::path::PathBuf,
};

#[derive(Default)]
pub struct AdvancedOpenWindow {
    pub open: WindowOpen,
    pub src_args: SourceArgs,
    pub path_to_meta: Option<PathBuf>,
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

impl super::Window for AdvancedOpenWindow {
    fn ui(
        &mut self,
        WinCtx {
            ui,
            gui,
            app,
            font_size,
            line_spacing,
            ..
        }: WinCtx,
    ) {
        let src_args = &mut self.src_args;
        ui.heading("Source");
        match &src_args.file {
            Some(file) => {
                ui.label(file.display().to_string());
            }
            None => {
                ui.label("<No file selected>");
            }
        }
        if ui.button("Select file...").clicked() {
            gui.fileops.advanced_open_pick_file();
        }
        opt(
            ui,
            &mut src_args.jump,
            "jump",
            "Jump to offset on startup",
            |ui, jump| {
                ui.add(egui::DragValue::new(jump));
            },
        );
        opt(
            ui,
            &mut src_args.hard_seek,
            "hard seek",
            "Seek to offset, consider it beginning of the file in the editor",
            |ui, hard_seek| {
                ui.add(egui::DragValue::new(hard_seek));
            },
        );
        opt(
            ui,
            &mut src_args.take,
            "take",
            "Read only this many bytes",
            |ui, take| {
                ui.add(egui::DragValue::new(take));
            },
        );
        ui.checkbox(&mut src_args.read_only, "read-only")
            .on_hover_text("Open file as read-only");
        if ui
            .checkbox(&mut src_args.stream, "stream")
            .on_hover_text(
                "Specify source as a streaming source (for example, standard streams).\n\
             Sets read-only attribute",
            )
            .changed()
        {
            src_args.read_only = src_args.stream;
        }
        ui.heading("Meta");
        match &self.path_to_meta {
            Some(file) => {
                ui.label(file.display().to_string());
            }
            None => {
                ui.label("<No meta file selected>");
            }
        }
        if ui.button("Select file...").clicked() {
            gui.fileops.advanced_open_pick_metafile();
        }
        ui.separator();
        if ui.add_enabled(src_args.file.is_some(), egui::Button::new("Load")).clicked() {
            msg_if_fail(
                app.load_file_args(
                    src_args.clone(),
                    self.path_to_meta.clone(),
                    &mut gui.msg_dialog,
                    font_size,
                    line_spacing,
                ),
                "Failed to load file",
                &mut gui.msg_dialog,
            );
            self.open.set(false);
        }
    }

    fn title(&self) -> &str {
        "Advanced open"
    }
}
