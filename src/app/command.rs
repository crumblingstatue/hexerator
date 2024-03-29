//! Due to various issues with overlapping borrows, it's not always feasible to do every operation
//! on the application state at the time the action is requested.
//!
//! Sometimes we need to wait until we have exclusive access to the application before we can
//! perform an operation.
//!
//! One possible way to do this is to encode whatever data an operation requires, and save it until
//! we have exclusive access, and then perform it.

use {
    super::App,
    crate::{
        meta::{NamedView, PerspectiveKey, RegionKey},
        view::{HexData, View, ViewKind},
    },
    std::collections::VecDeque,
};

pub enum Cmd {
    CreatePerspective {
        region_key: RegionKey,
        name: String,
    },
    RemovePerspective(PerspectiveKey),
    SetSelection(usize, usize),
    SetAndFocusCursor(usize),
    SetLayout(crate::meta::LayoutKey),
    FocusView(crate::meta::ViewKey),
    CreateView {
        perspective_key: PerspectiveKey,
        name: String,
    },
}

/// Application command queue.
///
/// Push operations with `push`, and call `App::flush_command_queue` when you have
/// exclusive access to the `App`.
///
/// `App::flush_command_queue` is called automatically every frame, if you don't need to perform the operations sooner.
#[derive(Default)]
pub struct CommandQueue {
    inner: VecDeque<Cmd>,
}

impl CommandQueue {
    pub fn push(&mut self, command: Cmd) {
        self.inner.push_back(command);
    }
}

impl App {
    /// Flush the [`CommandQueue`] and perform all operations queued up.
    ///
    /// Automatically called every frame, but can be called manually if operations need to be
    /// performed sooner.
    pub fn flush_command_queue(&mut self) {
        while let Some(cmd) = self.cmd.inner.pop_front() {
            perform_command(self, cmd);
        }
    }
}

fn perform_command(app: &mut App, cmd: Cmd) {
    match cmd {
        Cmd::CreatePerspective { region_key, name } => {
            app.add_perspective_from_region(region_key, name)
        }
        Cmd::SetSelection(a, b) => {
            app.hex_ui.select_a = Some(a);
            app.hex_ui.select_b = Some(b);
        }
        Cmd::SetAndFocusCursor(off) => {
            app.edit_state.cursor = off;
            app.center_view_on_offset(off);
            app.hex_ui.flash_cursor();
        }
        Cmd::SetLayout(key) => app.hex_ui.current_layout = key,
        Cmd::FocusView(key) => app.hex_ui.focused_view = Some(key),
        Cmd::RemovePerspective(key) => {
            app.meta_state.meta.low.perspectives.remove(key);
            // TODO: Should probably handle dangling keys somehow.
            // either by not allowing removal in that case, or being robust against dangling keys
            // or removing everything that uses a dangling key.
        }
        Cmd::CreateView {
            perspective_key,
            name,
        } => {
            app.meta_state.meta.views.insert(NamedView {
                view: View::new(ViewKind::Hex(HexData::default()), perspective_key),
                name,
            });
        }
    }
}
