use {
    super::{window_open::WindowOpen, WindowCtxt},
    crate::shell::msg_if_fail,
    anyhow::Context,
    std::{
        ffi::OsString,
        io::Read,
        process::{Child, Command, ExitStatus, Stdio},
    },
};

pub struct ExternalCommandWindow {
    pub open: WindowOpen,
    cmd_str: String,
    child: Option<Child>,
    exit_status: Option<ExitStatus>,
    err_msg: String,
    stdout: String,
    stderr: String,
    auto_exec: bool,
    inherited_streams: bool,
    selection_only: bool,
}

impl Default for ExternalCommandWindow {
    fn default() -> Self {
        Self {
            open: Default::default(),
            cmd_str: Default::default(),
            child: Default::default(),
            exit_status: Default::default(),
            err_msg: Default::default(),
            stdout: Default::default(),
            stderr: Default::default(),
            auto_exec: Default::default(),
            inherited_streams: Default::default(),
            selection_only: true,
        }
    }
}

enum Arg<'src> {
    TmpFilePath,
    Custom(&'src str),
}

impl ExternalCommandWindow {
    pub fn ui(&mut self, WindowCtxt { ui, gui, app, .. }: WindowCtxt) {
        let re = ui.add(
            egui::TextEdit::multiline(&mut self.cmd_str)
                .hint_text("Use {} to substitute filename.\nExample: aplay {} -f s16_le"),
        );
        if self.open.just_now() {
            re.request_focus();
        }
        ui.add_enabled(
            app.hex_ui.selection().is_some(),
            egui::Checkbox::new(&mut self.selection_only, "Selection only"),
        );
        ui.checkbox(&mut self.inherited_streams, "Inherited stdout/stderr")
            .on_hover_text(
                "Use this for large amounts of data that could block child processes, like music players, etc."
            );
        let exec_enabled = self.child.is_none();
        if ui.input(|inp| inp.key_pressed(egui::Key::Escape)) {
            self.open.set(false);
        }
        if ui
            .add_enabled(exec_enabled, egui::Button::new("Execute (ctrl+E)"))
            .clicked()
            || (exec_enabled
                && ((ui.input(|inp| {
                    inp.key_pressed(egui::Key::E) && inp.modifiers.ctrl && !self.open.just_now()
                })) || self.auto_exec))
        {
            let res: anyhow::Result<()> = try {
                // Parse args
                let (cmd, args) = parse(&self.cmd_str)?;
                // Generate temp file
                let range = if self.selection_only
                    && let Some(sel) = app.hex_ui.selection()
                {
                    sel.begin..=sel.end
                } else {
                    0..=app.data.len() - 1
                };
                let path = std::env::temp_dir().join("hexerator_data_tmp.bin");
                let data = app.data.get(range).context("Range out of bounds")?;
                std::fs::write(&path, data)?;
                // Spawn process
                let mut cmd = Command::new(cmd);
                cmd.args(resolve_args(args, &path));
                if self.inherited_streams {
                    cmd.stdout(Stdio::inherit());
                    cmd.stderr(Stdio::inherit());
                } else {
                    cmd.stdout(Stdio::piped());
                    cmd.stderr(Stdio::piped());
                }
                let handle = cmd.spawn()?;
                self.child = Some(handle);
            };
            msg_if_fail(res, "Failed to spawn command", &mut gui.msg_dialog);
        }
        ui.checkbox(&mut self.auto_exec, "Auto execute")
            .on_hover_text("Execute again after process finishes");
        if let Some(child) = &mut self.child {
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
                            self.stdout.clear();
                            if let Err(e) = stdout.read_to_string(&mut self.stdout) {
                                self.stdout = format!("<Error reading stdout: {e}>");
                            }
                        }
                        if let Some(stderr) = &mut child.stderr {
                            self.stderr.clear();
                            if let Err(e) = stderr.read_to_string(&mut self.stderr) {
                                self.stderr = format!("<Error reading stderr: {e}>");
                            }
                        }
                        self.child = None;
                        self.exit_status = Some(status)
                    }
                }
                Err(e) => self.err_msg = e.to_string(),
            }
        }
        if !self.err_msg.is_empty() {
            ui.label(egui::RichText::new(&self.err_msg).color(egui::Color32::RED));
        }
        if !self.stdout.is_empty() {
            ui.label("stdout");
            egui::ScrollArea::vertical()
                .id_source("stdout")
                .max_height(200.0)
                .show(ui, |ui| {
                    ui.text_edit_multiline(&mut &self.stdout[..]);
                });
        }
        if !self.stderr.is_empty() {
            ui.label("stderr");
            egui::ScrollArea::vertical()
                .id_source("stderr")
                .max_height(200.0)
                .show(ui, |ui| {
                    ui.text_edit_multiline(&mut &self.stderr[..]);
                });
        }
        self.open.post_ui();
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
