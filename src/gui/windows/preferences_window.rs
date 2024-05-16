use {
    super::{WinCtx, WindowOpen},
    crate::{
        app::App,
        config::{self, Config},
        gui::message_dialog::{Icon, MessageDialog},
    },
    egui_fontcfg::{CustomFontPaths, FontCfgUi, FontDefsUiMsg},
    egui_sfml::sfml::graphics::RenderWindow,
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
    fn ui(
        &mut self,
        WinCtx {
            ui, gui, app, rwin, ..
        }: WinCtx,
    ) {
        if self.open.just_now() {
            self.font_defs.families = app.cfg.font_families.clone();
            self.temp_custom_font_paths
                .clone_from(&app.cfg.custom_font_paths);
            let _ = egui_fontcfg::load_custom_fonts(
                &app.cfg.custom_font_paths,
                &mut self.font_defs.font_data,
            );
        }
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.tab, Tab::Video, Tab::Video.label());
            ui.selectable_value(&mut self.tab, Tab::Style, Tab::Style.label());
            ui.selectable_value(&mut self.tab, Tab::Fonts, Tab::Fonts.label());
        });
        ui.separator();
        match self.tab {
            Tab::Video => video_ui(ui, app, rwin),
            Tab::Style => style_ui(app, ui),
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

fn video_ui(ui: &mut egui::Ui, app: &mut App, rwin: &mut RenderWindow) {
    if ui.checkbox(&mut app.cfg.vsync, "Vsync").clicked() {
        rwin.set_vertical_sync_enabled(app.cfg.vsync);
    }
    ui.horizontal(|ui| {
        ui.label("FPS limit (0 to disable)");
        ui.add(egui::DragValue::new(&mut app.cfg.fps_limit));
        if ui.button("Set").clicked() {
            rwin.set_framerate_limit(app.cfg.fps_limit);
        }
    });
}

fn style_ui(app: &mut App, ui: &mut egui::Ui) {
    ui.group(|ui| {
        let style = &mut app.cfg.style;
        ui.heading("Font sizes");
        let mut any_changed = false;
        ui.horizontal(|ui| {
            ui.label("heading");
            any_changed |= ui
                .add(egui::DragValue::new(&mut style.font_sizes.heading).clamp_range(3..=100))
                .changed();
        });
        ui.horizontal(|ui| {
            ui.label("body");
            any_changed |= ui
                .add(egui::DragValue::new(&mut style.font_sizes.body).clamp_range(3..=100))
                .changed();
        });
        ui.horizontal(|ui| {
            ui.label("monospace");
            any_changed |= ui
                .add(egui::DragValue::new(&mut style.font_sizes.monospace).clamp_range(3..=100))
                .changed();
        });
        ui.horizontal(|ui| {
            ui.label("button");
            any_changed |= ui
                .add(egui::DragValue::new(&mut style.font_sizes.button).clamp_range(3..=100))
                .changed();
        });
        ui.horizontal(|ui| {
            ui.label("small");
            any_changed |= ui
                .add(egui::DragValue::new(&mut style.font_sizes.small).clamp_range(3..=100))
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
