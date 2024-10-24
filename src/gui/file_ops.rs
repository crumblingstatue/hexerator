use {
    crate::{
        app::App,
        args::SourceArgs,
        gui::{
            message_dialog::MessageDialog,
            windows::{AdvancedOpenWindow, FileDiffResultWindow},
        },
        meta::{region::Region, ViewKey},
        shell::{msg_fail, msg_if_fail},
        source::Source,
        value_color::{self, ColorMethod},
    },
    anyhow::Context as _,
    egui_file_dialog::FileDialog,
    std::{fs::OpenOptions, io::Write as _, path::Path},
};

pub struct FileOps {
    pub dialog: FileDialog,
    pub op: Option<FileOp>,
}

impl Default for FileOps {
    fn default() -> Self {
        Self {
            dialog: FileDialog::new()
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0., 0.))
                .allow_path_edit_to_save_file_without_extension(true),
            op: Default::default(),
        }
    }
}

#[derive(Debug)]
pub enum FileOp {
    LoadMetaFile,
    AdvancedOpenPickFile,
    AdvancedOpenPickMetafile,
    LoadFile,
    LoadPaletteForView(ViewKey),
    LoadPaletteFromImageForView(ViewKey),
    DiffWithFile,
    LoadLuaScript,
    SavePaletteForView(ViewKey),
    SaveFileAs,
    SaveLuaScript,
    SaveMetaFileAs,
    SaveSelectionToFile(Region),
}

impl FileOps {
    pub fn update(
        &mut self,
        ctx: &egui::Context,
        app: &mut App,
        msg: &mut MessageDialog,
        advanced_open_window: &mut AdvancedOpenWindow,
        file_diff_result_window: &mut FileDiffResultWindow,
        font_size: u16,
        line_spacing: u16,
    ) {
        self.dialog.update(ctx);
        if let Some(path) = self.dialog.take_selected()
            && let Some(op) = self.op.take()
        {
            match op {
                FileOp::LoadMetaFile => {
                    msg_if_fail(
                        app.consume_meta_from_file(path),
                        "Failed to load metafile",
                        msg,
                    );
                }
                FileOp::AdvancedOpenPickFile => {
                    advanced_open_window.src_args.file = Some(path);
                }
                FileOp::AdvancedOpenPickMetafile => {
                    advanced_open_window.path_to_meta = Some(path);
                }
                FileOp::LoadFile => {
                    let write = OpenOptions::new().write(true).open(&path).is_ok();
                    msg_if_fail(
                        app.load_file(path, !write, msg, font_size, line_spacing),
                        "Failed to load file (read-write)",
                        msg,
                    );
                }
                FileOp::LoadPaletteForView(key) => match value_color::load_palette(&path) {
                    Ok(pal) => {
                        let view = &mut app.meta_state.meta.views[key].view;
                        view.presentation.color_method = ColorMethod::Custom(Box::new(pal));
                    }
                    Err(e) => msg_fail(&e, "Failed to load pal", msg),
                },
                FileOp::LoadPaletteFromImageForView(key) => {
                    let view = &mut app.meta_state.meta.views[key].view;
                    let ColorMethod::Custom(pal) = &mut view.presentation.color_method else {
                        return;
                    };
                    let result: anyhow::Result<()> = try {
                        let img = image::open(path).context("Failed to load image")?.to_rgb8();
                        let (width, height) = (img.width(), img.height());
                        let sel = app.hex_ui.selection().context("Missing app selection")?;
                        let mut i = 0;
                        for y in 0..height {
                            for x in 0..width {
                                let &image::Rgb(rgb) = img.get_pixel(x, y);
                                let Some(byte) = app.data.get(sel.begin + i) else {
                                    break;
                                };
                                pal.0[*byte as usize] = rgb;
                                i += 1;
                            }
                        }
                    };
                    msg_if_fail(result, "Failed to load palette from reference image", msg);
                }
                FileOp::DiffWithFile => {
                    msg_if_fail(
                        app.diff_with_file(path, file_diff_result_window),
                        "Failed to diff",
                        msg,
                    );
                }
                FileOp::LoadLuaScript => {
                    let res: anyhow::Result<()> = try {
                        app.meta_state.meta.misc.exec_lua_script = std::fs::read_to_string(path)?;
                    };
                    msg_if_fail(res, "Failed to load script", msg);
                }
                FileOp::SavePaletteForView(key) => {
                    let view = &mut app.meta_state.meta.views[key].view;
                    let ColorMethod::Custom(pal) = &view.presentation.color_method else {
                        return;
                    };
                    msg_if_fail(
                        value_color::save_palette(pal, &path),
                        "Failed to save pal",
                        msg,
                    );
                }
                FileOp::SaveFileAs => {
                    let result: anyhow::Result<()> = try {
                        let mut f = std::fs::OpenOptions::new()
                            .create(true)
                            .truncate(true)
                            .read(true)
                            .write(true)
                            .open(&path)?;
                        f.write_all(&app.data)?;
                        app.source = Some(Source::file(f));
                        app.src_args.file = Some(path);
                        app.cfg.recent.use_(SourceArgs {
                            file: app.src_args.file.clone(),
                            jump: None,
                            hard_seek: None,
                            take: None,
                            read_only: false,
                            stream: false,
                        });
                    };
                    msg_if_fail(result, "Failed to save as", msg);
                }
                FileOp::SaveLuaScript => {
                    msg_if_fail(
                        std::fs::write(path, &app.meta_state.meta.misc.exec_lua_script),
                        "Failed to save script",
                        msg,
                    );
                }
                FileOp::SaveMetaFileAs => {
                    msg_if_fail(
                        app.save_meta_to_file(path, false),
                        "Failed to save metafile",
                        msg,
                    );
                }
                FileOp::SaveSelectionToFile(sel) => {
                    let result = std::fs::write(path, &app.data[sel.begin..=sel.end]);
                    msg_if_fail(result, "Failed to save selection to file", msg);
                }
            }
        }
    }
    pub fn load_file(&mut self, source_file: Option<&Path>) {
        if let Some(path) = source_file
            && let Some(parent) = path.parent()
        {
            let cfg = self.dialog.config_mut();
            parent.clone_into(&mut cfg.initial_directory);
        }
        self.dialog.select_file();
        self.op = Some(FileOp::LoadFile);
    }
    pub fn load_meta_file(&mut self) {
        self.dialog.select_file();
        self.op = Some(FileOp::LoadMetaFile);
    }

    pub fn advanced_open_pick_file(&mut self) {
        self.dialog.select_file();
        self.op = Some(FileOp::AdvancedOpenPickFile);
    }

    pub fn advanced_open_pick_metafile(&mut self) {
        self.dialog.select_file();
        self.op = Some(FileOp::AdvancedOpenPickMetafile);
    }

    pub fn load_palette_for_view(&mut self, key: ViewKey) {
        self.dialog.select_file();
        self.op = Some(FileOp::LoadPaletteForView(key));
    }

    pub fn load_palette_from_image_for_view(&mut self, view_key: ViewKey) {
        self.dialog.select_file();
        self.op = Some(FileOp::LoadPaletteFromImageForView(view_key));
    }

    pub fn diff_with_file(&mut self, source_file: Option<&Path>) {
        if let Some(path) = source_file
            && let Some(parent) = path.parent()
        {
            self.dialog.config_mut().initial_directory = parent.to_owned();
        }
        self.dialog.select_file();
        self.op = Some(FileOp::DiffWithFile);
    }

    pub fn load_lua_script(&mut self) {
        self.dialog.select_file();
        self.op = Some(FileOp::LoadLuaScript);
    }

    pub fn save_palette_for_view(&mut self, view_key: ViewKey) {
        self.dialog.save_file();
        self.op = Some(FileOp::SavePaletteForView(view_key));
    }

    pub(crate) fn save_file_as(&mut self) {
        self.dialog.save_file();
        self.op = Some(FileOp::SaveFileAs);
    }

    pub(crate) fn save_lua_script(&mut self) {
        self.dialog.save_file();
        self.op = Some(FileOp::SaveLuaScript);
    }

    pub(crate) fn save_metafile_as(&mut self) {
        self.dialog.save_file();
        self.op = Some(FileOp::SaveMetaFileAs);
    }

    pub(crate) fn save_selection_to_file(&mut self, region: Region) {
        self.dialog.save_file();
        self.op = Some(FileOp::SaveSelectionToFile(region));
    }
}
