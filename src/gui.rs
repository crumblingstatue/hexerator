pub use self::windows::ConMsg;
use {
    self::{
        command::GCommandQueue, file_ops::FileOps, inspect_panel::InspectPanel,
        message_dialog::MessageDialog, windows::Windows,
    },
    crate::{
        app::App,
        args::SourceArgs,
        config::Style,
        meta::{
            Bookmark,
            value_type::{self, ValueType},
        },
        view::{ViewportScalar, ViewportVec},
    },
    egui::{
        FontFamily::{self, Proportional},
        FontId,
        TextStyle::{Body, Button, Heading, Monospace, Small},
        TopBottomPanel, Window,
    },
    egui_colors::Colorix,
    egui_sfml::{SfEgui, sfml::graphics::RenderWindow},
    gamedebug_core::{IMMEDIATE, PERSISTENT},
    mlua::Lua,
    root_ctx_menu::ContextMenu,
    std::{
        any::TypeId,
        collections::{HashMap, HashSet},
    },
};

mod bottom_panel;
pub mod command;
pub mod dialogs;
mod egui_ui_ext;
pub mod file_ops;
pub mod inspect_panel;
pub mod message_dialog;
mod ops;
pub mod root_ctx_menu;
pub mod selection_menu;
pub mod top_menu;
mod top_panel;
pub mod windows;

type Dialogs = HashMap<TypeId, Box<dyn Dialog>>;

pub type HighlightSet = HashSet<usize>;

#[derive(Default)]
pub struct Gui {
    pub inspect_panel: InspectPanel,
    pub dialogs: Dialogs,
    pub context_menu: Option<ContextMenu>,
    pub msg_dialog: MessageDialog,
    /// What to highlight in addition to selection. Can be updated by various actions that want to highlight stuff
    pub highlight_set: HighlightSet,
    pub cmd: GCommandQueue,
    pub fileops: FileOps,
    pub win: Windows,
    pub colorix: Option<Colorix>,
}

pub trait Dialog {
    fn title(&self) -> &str;
    /// Do the ui for this dialog. Returns whether to keep this dialog open.
    fn ui(
        &mut self,
        ui: &mut egui::Ui,
        app: &mut App,
        gui: &mut crate::gui::Gui,
        lua: &Lua,
        font_size: u16,
        line_spacing: u16,
    ) -> bool;
    /// Called when dialog is opened. Can be used to set just-opened flag, etc.
    fn on_open(&mut self) {}
    fn has_close_button(&self) -> bool {
        false
    }
}

impl Gui {
    pub fn add_dialog<D: Dialog + 'static>(gui_dialogs: &mut Dialogs, mut dialog: D) {
        dialog.on_open();
        gui_dialogs.insert(TypeId::of::<D>(), Box::new(dialog));
    }
}

/// The bool indicates whether the application should continue running
pub fn do_egui(
    sf_egui: &mut SfEgui,
    gui: &mut crate::gui::Gui,
    app: &mut App,
    mouse_pos: ViewportVec,
    lua: &Lua,
    rwin: &mut RenderWindow,
    font_size: u16,
    line_spacing: u16,
) -> anyhow::Result<(egui_sfml::DrawInput, bool)> {
    let di = sf_egui.run(rwin, |_rwin, ctx| {
        let mut open = IMMEDIATE.enabled() || PERSISTENT.enabled();
        let was_open = open;
        if open {
            app.imm_debug_fun();
        }
        Window::new("Debug").open(&mut open).show(ctx, windows::debug_window::ui);
        if was_open && !open {
            IMMEDIATE.toggle();
            PERSISTENT.toggle();
        }
        gui.msg_dialog.show(ctx, &mut app.clipboard, &mut app.cmd);
        app.flush_command_queue(gui, lua, font_size, line_spacing);
        self::Windows::update(ctx, gui, app, lua, font_size, line_spacing);

        // Context menu
        if let Some(menu) = gui.context_menu.take() {
            if root_ctx_menu::show(&menu, ctx, app, gui) {
                std::mem::swap(&mut gui.context_menu, &mut Some(menu));
            }
        }
        // Panels
        let top_re = TopBottomPanel::top("top_panel").show(ctx, |ui| {
            top_panel::ui(ui, gui, app, lua, font_size, line_spacing);
        });
        let bot_re = TopBottomPanel::bottom("bottom_panel")
            .show(ctx, |ui| bottom_panel::ui(ui, app, mouse_pos, gui));
        let right_re = egui::SidePanel::right("right_panel")
            .show(ctx, |ui| inspect_panel::ui(ui, app, gui, mouse_pos))
            .response;
        let padding = 2;
        app.hex_ui.hex_iface_rect.x = padding;
        #[expect(
            clippy::cast_possible_truncation,
            reason = "Window size can't exceed i16"
        )]
        {
            app.hex_ui.hex_iface_rect.y = top_re.response.rect.bottom() as ViewportScalar + padding;
        }
        #[expect(
            clippy::cast_possible_truncation,
            reason = "Window size can't exceed i16"
        )]
        {
            app.hex_ui.hex_iface_rect.w = right_re.rect.left() as ViewportScalar - padding * 2;
        }
        #[expect(
            clippy::cast_possible_truncation,
            reason = "Window size can't exceed i16"
        )]
        {
            app.hex_ui.hex_iface_rect.h = (bot_re.response.rect.top() as ViewportScalar
                - app.hex_ui.hex_iface_rect.y)
                - padding * 2;
        }
        let mut dialogs = std::mem::take(&mut gui.dialogs);
        dialogs.retain(|_k, dialog| {
            let mut retain = true;
            let mut win = Window::new(dialog.title())
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0., 0.]);
            let mut open = true;
            if dialog.has_close_button() {
                win = win.open(&mut open);
            }
            win.show(ctx, |ui| {
                retain = dialog.ui(ui, app, gui, lua, font_size, line_spacing);
            });
            if !open {
                retain = false;
            }
            retain
        });
        std::mem::swap(&mut gui.dialogs, &mut dialogs);
        // File dialog
        gui.fileops.update(
            ctx,
            app,
            &mut gui.msg_dialog,
            &mut gui.win.advanced_open,
            &mut gui.win.file_diff_result,
            font_size,
            line_spacing,
        );
    })?;
    Ok((di, true))
}

pub fn set_font_sizes_ctx(ctx: &egui::Context, style: &Style) {
    let mut egui_style = (*ctx.style()).clone();
    set_font_sizes_style(&mut egui_style, style);
    ctx.set_style(egui_style);
}

pub fn set_font_sizes_style(egui_style: &mut egui::Style, style: &Style) {
    egui_style.text_styles = [
        (
            Heading,
            FontId::new(style.font_sizes.heading.into(), Proportional),
        ),
        (
            Body,
            FontId::new(style.font_sizes.body.into(), Proportional),
        ),
        (
            Monospace,
            FontId::new(style.font_sizes.monospace.into(), FontFamily::Monospace),
        ),
        (
            Button,
            FontId::new(style.font_sizes.button.into(), Proportional),
        ),
        (
            Small,
            FontId::new(style.font_sizes.small.into(), Proportional),
        ),
    ]
    .into();
}

fn opt<V: Default>(
    ui: &mut egui::Ui,
    val: &mut Option<V>,
    label: &str,
    desc: &str,
    f: impl FnOnce(&mut egui::Ui, &mut V),
) {
    ui.horizontal(|ui| {
        let mut checked = val.is_some();
        ui.checkbox(&mut checked, label).on_hover_text(desc);
        if checked {
            f(ui, val.get_or_insert_with(Default::default));
        } else {
            *val = None;
        }
    });
}

fn src_args_ui(ui: &mut egui::Ui, src_args: &mut SourceArgs) {
    opt(
        ui,
        &mut src_args.jump,
        "jump",
        "Jump to offset on startup",
        |ui, jump| {
            ui.add(egui::DragValue::new(jump));
        },
    );
    opt(
        ui,
        &mut src_args.hard_seek,
        "hard seek",
        "Seek to offset, consider it beginning of the file in the editor",
        |ui, hard_seek| {
            ui.add(egui::DragValue::new(hard_seek));
        },
    );
    opt(
        ui,
        &mut src_args.take,
        "take",
        "Read only this many bytes",
        |ui, take| {
            ui.add(egui::DragValue::new(take));
        },
    );
    ui.checkbox(&mut src_args.read_only, "read-only")
        .on_hover_text("Open file as read-only");
    if ui
        .checkbox(&mut src_args.stream, "stream")
        .on_hover_text(
            "Specify source as a streaming source (for example, standard streams).\n\
             Sets read-only attribute",
        )
        .changed()
    {
        src_args.read_only = src_args.stream;
    }
}

fn add_new_bookmark(app: &mut App, gui: &mut Gui, byte_off: usize) {
    let bms = &mut app.meta_state.meta.bookmarks;
    let idx = bms.len();
    bms.push(Bookmark {
        offset: byte_off,
        label: format!("New @ offset {byte_off}"),
        desc: String::new(),
        value_type: ValueType::U8(value_type::U8),
    });
    gui.win.bookmarks.open.set(true);
    gui.win.bookmarks.selected = Some(idx);
    gui.win.bookmarks.edit_name = true;
    gui.win.bookmarks.focus_text_edit = true;
}
