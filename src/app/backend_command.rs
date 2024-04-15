//! This module is similar in purpose to [`crate::app:command`].
//!
//! See that module for more information.

use {super::App, egui_sfml::sfml::graphics::RenderWindow, std::collections::VecDeque};

pub enum BackendCmd {
    SetWindowTitle(String),
}

/// Gui command queue.
///
/// Push operations with `push`, and call [`Gui::flush_command_queue`] when you have
/// exclusive access to the [`Gui`].
///
/// [`Gui::flush_command_queue`] is called automatically every frame, if you don't need to perform the operations sooner.
#[derive(Default)]
pub struct BackendCommandQueue {
    inner: VecDeque<BackendCmd>,
}

impl BackendCommandQueue {
    pub fn push(&mut self, command: BackendCmd) {
        self.inner.push_back(command);
    }
}

impl App {
    /// Flush the [`GCommandQueue`] and perform all operations queued up.
    ///
    /// Automatically called every frame, but can be called manually if operations need to be
    /// performed sooner.
    pub fn flush_backend_command_queue(&mut self, rw: &mut RenderWindow) {
        while let Some(cmd) = self.backend_cmd.inner.pop_front() {
            perform_command(cmd, rw);
        }
    }
}

fn perform_command(cmd: BackendCmd, rw: &mut RenderWindow) {
    match cmd {
        BackendCmd::SetWindowTitle(title) => rw.set_title(&title),
    }
}
