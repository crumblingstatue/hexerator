//! Due to various issues with overlapping borrows, it's not always feasible to do every operation
//! on the application state at the time the action is requested.
//!
//! Sometimes we need to wait until we have exclusive access to the application before we can
//! perform an operation.
//!
//! One possible way to do this is to encode whatever data an operation requires, and save it until
//! we have exclusive access, and then perform it.

use {
    super::{backend_command::BackendCmd, App},
    crate::{
        gui::Gui,
        meta::{NamedView, PerspectiveKey, RegionKey},
        scripting::exec_lua,
        shell::msg_if_fail,
        view::{HexData, View, ViewKind},
    },
    egui_sfml::sfml::graphics::Font,
    mlua::Lua,
    std::{collections::VecDeque, path::Path},
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
    /// Finish saving a truncated file
    SaveTruncateFinish,
    /// Extend (or truncate) the data buffer to a new length
    ExtendDocument {
        new_len: usize,
    },
    /// Paste bytes at the requested index
    PasteBytes {
        at: usize,
        bytes: Vec<u8>,
    },
    /// A new source was loaded, process the changes
    ProcessSourceChange,
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
    pub fn flush_command_queue(&mut self, gui: &mut Gui, lua: &Lua, font: &Font) {
        while let Some(cmd) = self.cmd.inner.pop_front() {
            perform_command(self, cmd, gui, lua, font);
        }
    }
}

/// Perform a command. Called by `App::flush_command_queue`, but can be called manually if you
/// have a `Cmd` you would like you perform.
pub fn perform_command(app: &mut App, cmd: Cmd, gui: &mut Gui, lua: &Lua, font: &Font) {
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
        Cmd::SaveTruncateFinish => {
            msg_if_fail(
                app.save_truncated_file_finish(),
                "Save error",
                &mut gui.msg_dialog,
            );
        }
        Cmd::ExtendDocument { new_len } => {
            app.data.resize(new_len, 0);
        }
        Cmd::PasteBytes { at, bytes } => {
            app.data[at..at + bytes.len()].copy_from_slice(&bytes);
        }
        Cmd::ProcessSourceChange => {
            app.backend_cmd.push(BackendCmd::SetWindowTitle(format!(
                "{} - Hexerator",
                app.source_file().map_or("no source", path_filename_as_str)
            )));
            if let Some(key) = &app.meta_state.meta.onload_script {
                let scr = &app.meta_state.meta.scripts[*key];
                let content = scr.content.clone();
                let result = exec_lua(lua, &content, app, gui, font, "", Some(*key));
                msg_if_fail(
                    result,
                    "Failed to execute onload lua script",
                    &mut gui.msg_dialog,
                );
            }
        }
    }
}

fn path_filename_as_str(path: &Path) -> &str {
    path.file_name()
        .map_or("<no_filename>", |osstr| osstr.to_str().unwrap_or_default())
}
