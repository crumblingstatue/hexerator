//! This module is similar in purpose to [`crate::app::command`].
//!
//! See that module for more information.

use {
    super::Gui,
    crate::shell::msg_fail,
    std::{collections::VecDeque, process::Command},
    sysinfo::ProcessesToUpdate,
};

pub enum GCmd {
    OpenPerspectiveWindow,
    /// Spawn a command with optional arguments. Must not be an empty vector.
    SpawnCommand {
        args: Vec<String>,
        /// If `Some`, don't focus a pid, just filter for this process in the list.
        ///
        /// The idea is that if your command spawns a child process, it might not spawn immediately,
        /// so the user can wait for it to appear on the process list, with the applied filter.
        look_for_proc: Option<String>,
    },
}

/// Gui command queue.
///
/// Push operations with `push`, and call [`Gui::flush_command_queue`] when you have
/// exclusive access to the [`Gui`].
///
/// [`Gui::flush_command_queue`] is called automatically every frame, if you don't need to perform the operations sooner.
#[derive(Default)]
pub struct GCommandQueue {
    inner: VecDeque<GCmd>,
}

impl GCommandQueue {
    pub fn push(&mut self, command: GCmd) {
        self.inner.push_back(command);
    }
}

impl Gui {
    /// Flush the [`GCommandQueue`] and perform all operations queued up.
    ///
    /// Automatically called every frame, but can be called manually if operations need to be
    /// performed sooner.
    pub fn flush_command_queue(&mut self) {
        while let Some(cmd) = self.cmd.inner.pop_front() {
            perform_command(self, cmd);
        }
    }
}

fn perform_command(gui: &mut Gui, cmd: GCmd) {
    match cmd {
        GCmd::OpenPerspectiveWindow => gui.win.perspectives.open.set(true),
        GCmd::SpawnCommand {
            mut args,
            look_for_proc,
        } => {
            let cmd = args.remove(0);
            match Command::new(cmd).args(args).spawn() {
                Ok(child) => {
                    gui.win.open_process.open.set(true);
                    match look_for_proc {
                        Some(procname) => {
                            gui.win
                                .open_process
                                .sys
                                .refresh_processes(ProcessesToUpdate::All, true);
                            gui.win.open_process.filters.proc_name = procname;
                        }
                        None => {
                            gui.win.open_process.selected_pid =
                                Some(sysinfo::Pid::from_u32(child.id()))
                        }
                    }
                }
                Err(e) => {
                    msg_fail(&e, "Failed to spawn command", &mut gui.msg_dialog);
                }
            }
        }
    }
}
