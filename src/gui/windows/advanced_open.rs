use {
    super::{WinCtx, WindowOpen},
    crate::args::SourceArgs,
    std::path::PathBuf,
};

#[derive(Default)]
pub struct AdvancedOpenWindow {
    pub open: WindowOpen,
    pub src_args: SourceArgs,
    pub path_to_meta: Option<PathBuf>,
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
        crate::gui::src_args_ui(ui, src_args);
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
            app.load_file_args(
                src_args.clone(),
                self.path_to_meta.clone(),
                &mut gui.msg_dialog,
                font_size,
                line_spacing,
            );
            self.open.set(false);
        }
    }

    fn title(&self) -> &str {
        "Advanced open"
    }
}
