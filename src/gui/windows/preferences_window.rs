use {
    super::{WinCtx, WindowOpen},
    crate::{
        app::{backend_command::BackendCmd, App},
        config::{self, Config, ProjectDirsExt},
        gui::message_dialog::{Icon, MessageDialog},
    },
    egui_colors::{tokens::ThemeColor, Colorix},
    egui_fontcfg::{CustomFontPaths, FontCfgUi, FontDefsUiMsg},
    rand::Rng,
};

#[derive(Default)]
pub struct PreferencesWindow {
    pub open: WindowOpen,
    tab: Tab,
    font_cfg: FontCfgUi,
    font_defs: egui::FontDefinitions,
    temp_custom_font_paths: CustomFontPaths,
}

#[derive(Default, PartialEq)]
enum Tab {
    #[default]
    Video,
    Style,
    Fonts,
}

impl Tab {
    fn label(&self) -> &'static str {
        match self {
            Tab::Video => "Video",
            Tab::Style => "Style",
            Tab::Fonts => "Fonts",
        }
    }
}

impl super::Window for PreferencesWindow {
    fn ui(&mut self, WinCtx { ui, gui, app, .. }: WinCtx) {
        if self.open.just_now() {
            self.font_defs = ui.ctx().fonts(|f| f.lock().fonts.definitions().clone());
            self.temp_custom_font_paths.clone_from(&app.cfg.custom_font_paths);
            let _ = egui_fontcfg::load_custom_fonts(
                &app.cfg.custom_font_paths,
                &mut self.font_defs.font_data,
            );
        }
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.tab, Tab::Video, Tab::Video.label());
            ui.selectable_value(&mut self.tab, Tab::Style, Tab::Style.label());
            ui.selectable_value(&mut self.tab, Tab::Fonts, Tab::Fonts.label());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Open config dir").clicked() {
                    match crate::config::project_dirs() {
                        Some(dirs) => {
                            if let Err(e) = open::that(dirs.config_dir()) {
                                gui.msg_dialog.open(
                                    Icon::Error,
                                    "Error opening config dir",
                                    e.to_string(),
                                );
                            }
                        }
                        None => gui.msg_dialog.open(
                            Icon::Error,
                            "Error opening config dir",
                            "Missing config dir",
                        ),
                    }
                }
            });
        });
        ui.separator();
        match self.tab {
            Tab::Video => video_ui(ui, app),
            Tab::Style => style_ui(app, ui, &mut gui.colorix, &mut gui.msg_dialog),
            Tab::Fonts => fonts_ui(
                ui,
                &mut self.font_cfg,
                &mut self.font_defs,
                &mut app.cfg,
                &mut self.temp_custom_font_paths,
                &mut gui.msg_dialog,
            ),
        }
    }

    fn title(&self) -> &str {
        "Preferences"
    }
}

fn video_ui(ui: &mut egui::Ui, app: &mut App) {
    if ui.checkbox(&mut app.cfg.vsync, "Vsync").clicked() {
        app.backend_cmd.push(BackendCmd::ApplyVsyncCfg);
    }
    ui.horizontal(|ui| {
        ui.label("FPS limit (0 to disable)");
        ui.add(egui::DragValue::new(&mut app.cfg.fps_limit));
        if ui.button("Set").clicked() {
            app.backend_cmd.push(BackendCmd::ApplyFpsLimit);
        }
    });
}

fn style_ui(
    app: &mut App,
    ui: &mut egui::Ui,
    opt_colorix: &mut Option<Colorix>,
    msg_dia: &mut MessageDialog,
) {
    ui.group(|ui| {
        let style = &mut app.cfg.style;
        ui.heading("Font sizes");
        let mut any_changed = false;
        ui.horizontal(|ui| {
            ui.label("heading");
            any_changed |= ui
                .add(egui::DragValue::new(&mut style.font_sizes.heading).range(3..=100))
                .changed();
        });
        ui.horizontal(|ui| {
            ui.label("body");
            any_changed |= ui
                .add(egui::DragValue::new(&mut style.font_sizes.body).range(3..=100))
                .changed();
        });
        ui.horizontal(|ui| {
            ui.label("monospace");
            any_changed |= ui
                .add(egui::DragValue::new(&mut style.font_sizes.monospace).range(3..=100))
                .changed();
        });
        ui.horizontal(|ui| {
            ui.label("button");
            any_changed |= ui
                .add(egui::DragValue::new(&mut style.font_sizes.button).range(3..=100))
                .changed();
        });
        ui.horizontal(|ui| {
            ui.label("small");
            any_changed |= ui
                .add(egui::DragValue::new(&mut style.font_sizes.small).range(3..=100))
                .changed();
        });
        if ui.button("Reset default").clicked() {
            *style = config::Style::default();
            any_changed = true;
        }
        if any_changed {
            crate::gui::set_font_sizes_ctx(ui.ctx(), style);
        }
    });
    ui.group(|ui| {
        let colorix = match opt_colorix {
            Some(colorix) => colorix,
            None => {
                if ui.button("Activate custom colors").clicked() {
                    opt_colorix.insert(Colorix::init(ui.ctx(), egui_colors::utils::EGUI_THEME))
                } else {
                    return;
                }
            }
        };
        let mut clear = false;
        ui.horizontal(|ui| {
            colorix.themes_dropdown(ui, None, false);
            ui.group(|ui| {
                ui.label("light dark toggle");
                colorix.light_dark_toggle_button(ui);
            });
            if ui.button("Random theme").clicked() {
                let mut rng = rand::thread_rng();
                *colorix = Colorix::init(
                    ui.ctx(),
                    std::array::from_fn(|_| ThemeColor::Custom(rng.gen::<[u8; 3]>())),
                );
            }
        });
        ui.separator();
        colorix.ui_combo_12(ui);
        if let Some(dirs) = crate::config::project_dirs() {
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Save").clicked() {
                    let data: [[u8; 3]; 12] = colorix.theme().map(|theme| theme.rgb());
                    if let Err(e) = std::fs::write(dirs.color_theme_path(), data.as_flattened()) {
                        msg_dia.open(Icon::Error, "Failed to save theme", e.to_string());
                    }
                };
                if ui.button("Remove custom colors").clicked() {
                    if let Err(e) = std::fs::remove_file(dirs.color_theme_path()) {
                        msg_dia.open(Icon::Error, "Failed to delete theme file", e.to_string());
                    }
                    clear = true;
                }
            });
        }
        if clear {
            ui.ctx().set_visuals(egui::Visuals::dark());
            *opt_colorix = None;
        }
    });
}

fn fonts_ui(
    ui: &mut egui::Ui,
    font_cfg_ui: &mut FontCfgUi,
    font_defs: &mut egui::FontDefinitions,
    cfg: &mut Config,
    temp_custom_font_paths: &mut CustomFontPaths,
    msg_dia: &mut MessageDialog,
) {
    let msg = font_cfg_ui.show(ui, font_defs, Some(temp_custom_font_paths));
    if matches!(msg, FontDefsUiMsg::SaveRequest) {
        cfg.font_families = font_defs.families.clone();
        cfg.custom_font_paths.clone_from(temp_custom_font_paths);
        msg_dia.open(
            Icon::Info,
            "Config saved",
            "Your font configuration has been saved.",
        );
    }
}
