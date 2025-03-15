use {
    super::{WinCtx, WindowOpen},
    crate::shell::msg_if_fail,
    egui_extras::{Column, TableBuilder},
    std::fmt::Write as _,
    sysinfo::System,
};

type InfoPair = (&'static str, String);

pub struct AboutWindow {
    pub open: WindowOpen,
    sys: System,
    info: [InfoPair; 14],
    os_name: String,
    os_ver: String,
}

impl Default for AboutWindow {
    fn default() -> Self {
        Self {
            open: Default::default(),
            sys: Default::default(),
            info: Default::default(),
            os_name: System::name().unwrap_or_else(|| "Unknown".into()),
            os_ver: System::os_version().unwrap_or_else(|| "Unknown version".into()),
        }
    }
}

const MIB: u64 = 1_048_576;

macro_rules! optenv {
    ($name:literal) => {
        option_env!($name).unwrap_or("<unavailable>").to_string()
    };
}

impl super::Window for AboutWindow {
    fn ui(&mut self, WinCtx { ui, gui, app, .. }: WinCtx) {
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
        if self.open.just_now() {
            self.sys.refresh_cpu_all();
            self.sys.refresh_memory();
            self.info = [
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
                    optenv!("VERGEN_BUILD_TIMESTAMP").split('T').next().unwrap_or("error").into(),
                ),
                ("Target", optenv!("VERGEN_CARGO_TARGET_TRIPLE")),
                ("Debug", optenv!("VERGEN_CARGO_DEBUG")),
                ("Opt-level", optenv!("VERGEN_CARGO_OPT_LEVEL")),
                ("Built with rustc", optenv!("VERGEN_RUSTC_SEMVER")),
                ("System", String::new()),
                ("OS", format!("{} {}", self.os_name, self.os_ver)),
                (
                    "Total memory",
                    format!("{} MiB", self.sys.total_memory() / MIB),
                ),
                (
                    "Used memory",
                    format!("{} MiB", self.sys.used_memory() / MIB),
                ),
                (
                    "Available memory",
                    format!("{} MiB", self.sys.available_memory() / MIB),
                ),
            ];
        }
        info_table(ui, &self.info);
        ui.separator();
        ui.vertical_centered_justified(|ui| {
            if ui.button("Copy to clipboard").clicked() {
                crate::app::set_clipboard_string(
                    &mut app.clipboard,
                    &mut gui.msg_dialog,
                    &clipfmt_info(&self.info),
                );
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
                self.open.set(false);
            }
        });
    }

    fn title(&self) -> &str {
        "About Hexerator"
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
                    });
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
