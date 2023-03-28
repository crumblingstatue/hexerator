use {
    crate::{
        app::App,
        gui::{window_open::WindowOpen, Gui},
        shell::msg_if_fail,
    },
    egui_extras::{Column, TableBuilder},
    std::fmt::Write,
    sysinfo::{CpuExt, System, SystemExt},
};

type InfoPair = (&'static str, String);

#[derive(Default)]
pub struct AboutWindow {
    pub open: WindowOpen,
    sys: System,
    info: [InfoPair; 15],
}

const MIB: u64 = 1_048_576;

macro_rules! optenv {
    ($name:literal) => {
        option_env!($name).unwrap_or("<unavailable>").to_string()
    };
}

impl AboutWindow {
    pub fn ui(ui: &mut egui::Ui, gui: &mut Gui, app: &mut App) {
        let win = &mut gui.about_window;
        if win.open.just_now() {
            win.sys.refresh_cpu();
            win.sys.refresh_memory();
            let system_name = win.sys.name().unwrap_or_else(|| "Unknown".into());
            let os_ver = win
                .sys
                .os_version()
                .unwrap_or_else(|| "Unknown version".into());
            win.info = [
                ("Hexerator", String::new()),
                ("Version", optenv!("CARGO_PKG_VERSION")),
                ("Git SHA", optenv!("VERGEN_GIT_SHA")),
                (
                    "Commit date",
                    optenv!("VERGEN_GIT_COMMIT_TIMESTAMP")
                        .split('T')
                        .next()
                        .unwrap_or("error")
                        .into(),
                ),
                (
                    "Build date",
                    optenv!("VERGEN_BUILD_TIMESTAMP")
                        .split('T')
                        .next()
                        .unwrap_or("error")
                        .into(),
                ),
                ("Target", optenv!("VERGEN_CARGO_TARGET_TRIPLE")),
                ("Debug", optenv!("VERGEN_CARGO_DEBUG")),
                ("Opt-level", optenv!("VERGEN_CARGO_OPT_LEVEL")),
                ("Built with rustc", optenv!("VERGEN_RUSTC_SEMVER")),
                ("System", String::new()),
                ("OS", format!("{system_name} {os_ver}")),
                ("CPU", win.sys.global_cpu_info().brand().into()),
                (
                    "Total memory",
                    format!("{} MiB", win.sys.total_memory() / MIB),
                ),
                (
                    "Used memory",
                    format!("{} MiB", win.sys.used_memory() / MIB),
                ),
                (
                    "Available memory",
                    format!("{} MiB", win.sys.available_memory() / MIB),
                ),
            ];
        }
        info_table(ui, &win.info);
        ui.separator();
        ui.vertical_centered_justified(|ui| {
            if ui.button("Copy to clipboard").clicked() {
                crate::app::set_clipboard_string(
                    &mut app.clipboard,
                    &mut gui.msg_dialog,
                    &clipfmt_info(&win.info),
                );
            }
        });
        ui.separator();
        ui.heading("Links");
        win.open.post_ui();
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

fn info_table(ui: &mut egui::Ui, info: &[InfoPair]) {
    ui.push_id(info.as_ptr(), |ui| {
        let body_height = ui.text_style_height(&egui::TextStyle::Body);
        TableBuilder::new(ui)
            .column(Column::auto())
            .column(Column::remainder())
            .resizable(true)
            .striped(true)
            .body(|mut body| {
                for (k, v) in info {
                    body.row(body_height + 2.0, |mut row| {
                        row.col(|ui| {
                            ui.label(*k);
                        });
                        row.col(|ui| {
                            ui.label(v);
                        });
                    })
                }
            });
    });
}

fn clipfmt_info(info: &[InfoPair]) -> String {
    let mut out = String::new();
    for (k, v) in info {
        let _ = writeln!(out, "{k}: {v}");
    }
    out
}
