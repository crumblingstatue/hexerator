//! This module is similar in purpose to [`crate::app:command`].
//!
//! See that module for more information.

use {super::Gui, std::collections::VecDeque};

pub enum GCmd {
    OpenPerspectiveWindow,
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
        GCmd::OpenPerspectiveWindow => gui.perspectives_window.open.set(true),
    }
}
