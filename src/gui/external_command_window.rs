use {
    super::{window_open::WindowOpen, Gui},
    crate::{app::App, shell::msg_if_fail},
    anyhow::Context,
    std::{
        ffi::OsString,
        io::Read,
        process::{Child, Command, ExitStatus, Stdio},
    },
};

#[derive(Default)]
pub struct ExternalCommandWindow {
    pub open: WindowOpen,
    cmd_str: String,
    child: Option<Child>,
    exit_status: Option<ExitStatus>,
    err_msg: String,
    stdout: String,
    stderr: String,
    auto_exec: bool,
}

enum Arg<'src> {
    TmpFilePath,
    Custom(&'src str),
}

impl ExternalCommandWindow {
    pub fn ui(ui: &mut egui::Ui, gui: &mut Gui, app: &mut App) {
        let win = &mut gui.external_command_window;
        ui.add(
            egui::TextEdit::multiline(&mut win.cmd_str)
                .hint_text("Use {} to substitute filename.\nExample: aplay {} -f s16_le"),
        );
        let exec_enabled = win.child.is_none();

        if ui
            .add_enabled(exec_enabled, egui::Button::new("Execute (ctrl+E)"))
            .clicked()
            || (exec_enabled
                && ((ui.input().key_pressed(egui::Key::E) && ui.input().modifiers.ctrl)
                    || win.auto_exec))
        {
            let res: anyhow::Result<()> = try {
                // Parse args
                let (cmd, args) = parse(&win.cmd_str)?;
                // Generate temp file
                let range = if let Some(sel) = app.hex_ui.selection() {
                    sel.begin..=sel.end
                } else {
                    0..=app.data.len() - 1
                };
                let path = std::env::temp_dir().join("hexerator_data_tmp.bin");
                std::fs::write(&path, &app.data[range])?;
                // Spawn process
                let handle = Command::new(cmd)
                    .args(resolve_args(args, &path))
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()?;
                win.child = Some(handle);
            };
            msg_if_fail(res, "Failed to spawn command", &mut gui.msg_dialog);
        }
        ui.checkbox(&mut win.auto_exec, "Auto execute")
            .on_hover_text("Execute again after process finishes");
        if let Some(child) = &mut win.child {
            ui.horizontal(|ui| {
                ui.label(format!("{} running", child.id()));
                if ui.button("Kill").clicked() {
                    msg_if_fail(child.kill(), "Failed to kill child", &mut gui.msg_dialog);
                }
            });
            match child.try_wait() {
                Ok(opt_status) => {
                    if let Some(status) = opt_status {
                        if let Some(stdout) = &mut child.stdout {
                            win.stdout.clear();
                            if let Err(e) = stdout.read_to_string(&mut win.stdout) {
                                win.stdout = format!("<Error reading stdout: {}>", e);
                            }
                        }
                        if let Some(stderr) = &mut child.stderr {
                            win.stderr.clear();
                            if let Err(e) = stderr.read_to_string(&mut win.stderr) {
                                win.stderr = format!("<Error reading stderr: {}>", e);
                            }
                        }
                        win.child = None;
                        win.exit_status = Some(status)
                    }
                }
                Err(e) => win.err_msg = e.to_string(),
            }
        }
        if !win.err_msg.is_empty() {
            ui.label(egui::RichText::new(&win.err_msg).color(egui::Color32::RED));
        }
        if !win.stdout.is_empty() {
            ui.label("stdout");
            egui::ScrollArea::vertical()
                .id_source("stdout")
                .max_height(200.0)
                .show(ui, |ui| {
                    ui.text_edit_multiline(&mut &win.stdout[..]);
                });
        }
        if !win.stderr.is_empty() {
            ui.label("stderr");
            egui::ScrollArea::vertical()
                .id_source("stderr")
                .max_height(200.0)
                .show(ui, |ui| {
                    ui.text_edit_multiline(&mut &win.stderr[..]);
                });
        }
    }
}

fn resolve_args<'src>(
    args: impl Iterator<Item = Arg<'src>> + 'src,
    path: &'src std::path::PathBuf,
) -> impl Iterator<Item = OsString> + 'src {
    args.map(|arg| match arg {
        Arg::TmpFilePath => path.into(),
        Arg::Custom(c) => c.into(),
    })
}

fn parse(input: &str) -> anyhow::Result<(&str, impl Iterator<Item = Arg>)> {
    let mut tokens = input.split_whitespace();
    let cmd = tokens.next().context("Missing command")?;
    let iter = tokens.map(|tok| {
        if tok == "{}" {
            Arg::TmpFilePath
        } else {
            Arg::Custom(tok)
        }
    });
    Ok((cmd, iter))
}
