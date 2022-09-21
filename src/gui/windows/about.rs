use {
    crate::{
        gui::{window_open::WindowOpen, Gui},
        shell::msg_if_fail,
    },
    egui_sfml::egui,
};

#[derive(Default)]
pub struct AboutWindow {
    pub open: WindowOpen,
}

impl AboutWindow {
    pub fn ui(ui: &mut egui::Ui, gui: &mut Gui) {
        ui.heading("Hexerator");
        ui.vertical_centered_justified(|ui| {
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
        });
        ui.separator();
        ui.heading("Links");
        ui.vertical_centered_justified(|ui| {
            let result: anyhow::Result<()> = try {
                if ui.link("📖 Book").clicked() {
                    open::that("https://crumblingstatue.github.io/hexerator-book/")?;
                }
                if ui.link(" Git repository").clicked() {
                    open::that("https://github.com/crumblingstatue/hexerator/")?;
                }
                if ui.link("💬 Discussions forum").clicked() {
                    open::that("https://github.com/crumblingstatue/hexerator/discussions")?;
                }
            };
            msg_if_fail(result, "Failed to open link", &mut gui.msg_dialog);
            ui.separator();
            if ui.button("Close").clicked() {
                gui.about_window.open.set(false);
            }
        });
    }
}
