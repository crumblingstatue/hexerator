use {
    crate::{
        gui::{window_open::WindowOpen, Gui},
        shell::msg_if_fail,
    },
    egui_sfml::egui,
    sysinfo::{CpuExt, System, SystemExt},
};

#[derive(Default)]
pub struct AboutWindow {
    pub open: WindowOpen,
    sys: System,
    system_name: String,
    os_ver: String,
}

impl AboutWindow {
    pub fn ui(ui: &mut egui::Ui, gui: &mut Gui) {
        let win = &mut gui.about_window;
        if win.open.just_now() {
            win.sys.refresh_cpu();
            win.sys.refresh_memory();
            win.system_name = win.sys.name().unwrap_or_else(|| "Unknown".into());
            win.os_ver = win
                .sys
                .os_version()
                .unwrap_or_else(|| "Unknown version".into());
        }
        ui.heading("Hexerator");
        ui.vertical_centered_justified(|ui| {
            let info = format!(
                "Version: {}\n\n\
                 Git SHA: {}\n\n\
                 Commit date: {}\n\n\
                 Build date: {}\n\n\
                 Target: {}\n\n\
                 Cargo profile: {}\n\n\
                 Built with rustc {}\n",
                env!("VERGEN_GIT_SEMVER"),
                env!("VERGEN_GIT_SHA"),
                env!("VERGEN_GIT_COMMIT_TIMESTAMP")
                    .split('T')
                    .next()
                    .unwrap_or("error"),
                env!("VERGEN_BUILD_TIMESTAMP")
                    .split('T')
                    .next()
                    .unwrap_or("error"),
                env!("VERGEN_CARGO_TARGET_TRIPLE"),
                env!("VERGEN_CARGO_PROFILE"),
                env!("VERGEN_RUSTC_SEMVER"),
            );
            ui.label(&info);
            if ui.button("Copy to clipboard").clicked() {
                ui.output().copied_text = info;
            }
        });
        ui.separator();
        ui.heading("System");
        ui.vertical_centered_justified(|ui| {
            let cpu = win.sys.global_cpu_info();
            const MIB: u64 = 1_048_576;
            ui.label(format!(
                "\
                OS: {} {}\n\
                CPU: {}\n\
                Total memory: {} MiB\n\
                Used memory: {} MiB\n\
                Available memory: {} MiB\n\
                ",
                win.system_name,
                win.os_ver,
                cpu.brand(),
                win.sys.total_memory() / MIB,
                win.sys.used_memory() / MIB,
                win.sys.available_memory() / MIB,
            ));
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
