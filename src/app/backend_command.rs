//! This module is similar in purpose to [`crate::app::command`].
//!
//! See that module for more information.

use {
    super::App, crate::config::Config, egui_sf2g::sf2g::graphics::RenderWindow,
    std::collections::VecDeque,
};

pub enum BackendCmd {
    SetWindowTitle(String),
    ApplyVsyncCfg,
    ApplyFpsLimit,
}

/// Gui command queue.
///
/// Push operations with `push`, and call [`App::flush_backend_command_queue`] when you have
/// exclusive access to the [`App`].
///
/// [`App::flush_backend_command_queue`] is called automatically every frame, if you don't need to perform the operations sooner.
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
    /// Flush the [`BackendCommandQueue`] and perform all operations queued up.
    ///
    /// Automatically called every frame, but can be called manually if operations need to be
    /// performed sooner.
    pub fn flush_backend_command_queue(&mut self, rw: &mut RenderWindow) {
        while let Some(cmd) = self.backend_cmd.inner.pop_front() {
            perform_command(cmd, rw, &self.cfg);
        }
    }
}

fn perform_command(cmd: BackendCmd, rw: &mut RenderWindow, cfg: &Config) {
    match cmd {
        BackendCmd::SetWindowTitle(title) => rw.set_title(&title),
        BackendCmd::ApplyVsyncCfg => {
            rw.set_vertical_sync_enabled(cfg.vsync);
        }
        BackendCmd::ApplyFpsLimit => {
            rw.set_framerate_limit(cfg.fps_limit);
        }
    }
}
