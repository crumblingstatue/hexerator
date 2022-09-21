use {crate::gui::window_open::WindowOpen, egui_sfml::egui};

#[derive(Default)]
pub struct AboutWindow {
    pub open: WindowOpen,
}

impl AboutWindow {
    pub fn ui(ui: &mut egui::Ui) {
        let info = format!(
            "Version: {}\n\n\
             Git SHA: {}\n\n\
             Built with rustc {}\n",
            env!("VERGEN_GIT_SEMVER"),
            env!("VERGEN_GIT_SHA"),
            env!("VERGEN_RUSTC_SEMVER"),
        );
        ui.label(&info);
        if ui.button("Copy to clipboard").clicked() {
            ui.output().copied_text = info;
        }
    }
}
