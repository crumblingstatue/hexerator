pub use self::windows::ConMsg;
use {
    self::{
        advanced_open_window::AdvancedOpenWindow,
        bookmarks_window::BookmarksWindow,
        command::GCommandQueue,
        external_command_window::ExternalCommandWindow,
        file_diff_result_window::FileDiffResultWindow,
        file_ops::FileOps,
        find_dialog::FindDialog,
        find_memory_pointers_window::FindMemoryPointersWindow,
        inspect_panel::InspectPanel,
        layouts_window::LayoutsWindow,
        message_dialog::MessageDialog,
        meta_diff_window::MetaDiffWindow,
        open_process_window::OpenProcessWindow,
        perspectives_window::PerspectivesWindow,
        preferences_window::PreferencesWindow,
        regions_window::RegionsWindow,
        views_window::ViewsWindow,
        windows::{
            LuaConsoleWindow, LuaHelpWindow, LuaWatchWindow, ScriptManagerWindow, VarsWindow,
        },
    },
    crate::{
        app::App,
        config::Style,
        gui::windows::AboutWindow,
        meta::{
            value_type::{ValueType, U8},
            Bookmark, ViewKey,
        },
        view::{ViewportScalar, ViewportVec},
    },
    egui_sfml::{
        egui::{
            FontFamily::{self, Proportional},
            FontId,
            TextStyle::{Body, Button, Heading, Monospace, Small},
            TopBottomPanel, Window,
        },
        sfml::graphics::{Font, RenderWindow},
        SfEgui,
    },
    gamedebug_core::{IMMEDIATE, PERSISTENT},
    mlua::Lua,
    std::{
        any::TypeId,
        collections::{HashMap, HashSet},
    },
};

mod advanced_open_window;
mod bookmarks_window;
mod bottom_panel;
pub mod command;
mod debug_window;
pub mod dialogs;
mod external_command_window;
pub mod file_diff_result_window;
pub mod file_ops;
pub mod find_dialog;
mod find_memory_pointers_window;
pub mod inspect_panel;
mod layouts_window;
pub mod message_dialog;
mod meta_diff_window;
mod open_process_window;
mod ops;
mod perspectives_window;
mod preferences_window;
mod regions_window;
pub mod selection_menu;
pub mod top_menu;
mod top_panel;
mod views_window;
mod window_open;
mod windows;

type Dialogs = HashMap<TypeId, Box<dyn Dialog>>;

pub type HighlightSet = HashSet<usize>;

#[derive(Default)]
pub struct Windows {
    pub layouts: LayoutsWindow,
    pub views: ViewsWindow,
    pub regions: RegionsWindow,
    pub bookmarks: BookmarksWindow,
    pub find: FindDialog,
    pub perspectives: PerspectivesWindow,
    pub file_diff_result: FileDiffResultWindow,
    pub open_process: OpenProcessWindow,
    pub find_memory_pointers: FindMemoryPointersWindow,
    pub advanced_open: AdvancedOpenWindow,
    pub external_command: ExternalCommandWindow,
    pub preferences: PreferencesWindow,
    pub about: AboutWindow,
    pub vars: VarsWindow,
    pub lua_help: LuaHelpWindow,
    pub lua_console: LuaConsoleWindow,
    pub lua_watch: Vec<LuaWatchWindow>,
    pub script_manager: ScriptManagerWindow,
    pub meta_diff: MetaDiffWindow,
}

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
}

pub struct ContextMenu {
    pos: egui::Pos2,
    data: ContextMenuData,
}

impl ContextMenu {
    pub fn new(mx: ViewportScalar, my: ViewportScalar, data: ContextMenuData) -> Self {
        Self {
            pos: egui::pos2(f32::from(mx), f32::from(my)),
            data,
        }
    }
}

pub struct ContextMenuData {
    pub view: Option<ViewKey>,
    pub byte_off: Option<usize>,
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
        font: &Font,
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

pub struct WindowCtxt<'a> {
    ui: &'a mut egui::Ui,
    gui: &'a mut crate::gui::Gui,
    app: &'a mut crate::app::App,
    rwin: &'a mut RenderWindow,
    lua: &'a mlua::Lua,
    font: &'a Font,
}

#[must_use = "Returns false if application should quit"]
pub fn do_egui(
    sf_egui: &mut SfEgui,
    gui: &mut crate::gui::Gui,
    app: &mut App,
    mouse_pos: ViewportVec,
    font: &Font,
    lua: &Lua,
    rwin: &mut RenderWindow,
) -> anyhow::Result<bool> {
    sf_egui.begin_frame();
    let ctx = sf_egui.context();

    let mut open = IMMEDIATE.enabled() || PERSISTENT.enabled();
    let was_open = open;
    Window::new("Debug")
        .open(&mut open)
        .show(ctx, debug_window::ui);
    if was_open && !open {
        IMMEDIATE.toggle();
        PERSISTENT.toggle();
    }
    gui.msg_dialog.show(ctx, &mut app.clipboard, &mut app.cmd);
    app.flush_command_queue(gui, lua, font);
    macro_rules! windows {
            ($($title:expr, $field:ident, $ty:ty;)*) => {
                $(
                    open = gui.win.$field.open.is();
                    Window::new($title).open(&mut open).show(ctx, |ui| <$ty>::ui(WindowCtxt{ ui, gui, app, rwin, lua, font }));
                    if !open {
                        gui.win.$field.open.set(false);
                    }
                )*
            };
        }
    windows! {
        "Find",                    find,                 FindDialog;
        "Regions",                 regions,              RegionsWindow;
        "Bookmarks",               bookmarks,            BookmarksWindow;
        "Layouts",                 layouts,              LayoutsWindow;
        "Views",                   views,                ViewsWindow;
        "Variables",               vars,                 VarsWindow;
        "Perspectives",            perspectives,         PerspectivesWindow;
        "File Diff results",       file_diff_result,     FileDiffResultWindow;
        "Diff against clean meta", meta_diff,            MetaDiffWindow;
        "Open process",            open_process,         OpenProcessWindow;
        "Find memory pointers",    find_memory_pointers, FindMemoryPointersWindow;
        "Advanced open",           advanced_open,        AdvancedOpenWindow;
        "External command",        external_command,     ExternalCommandWindow;
        "Preferences",             preferences,          PreferencesWindow;
        "Lua help",                lua_help,             LuaHelpWindow;
        "Lua console",             lua_console,          LuaConsoleWindow;
        "Script manager",          script_manager,       ScriptManagerWindow;
        "About Hexerator",         about,                AboutWindow;
    }

    let mut watch_windows = std::mem::take(&mut gui.win.lua_watch);
    let mut i = 0;
    watch_windows.retain_mut(|win| {
        let mut retain = true;
        Window::new(&win.name)
            .id(egui::Id::new("watch_w").with(i))
            .open(&mut retain)
            .show(ctx, |ui| win.ui(ui, gui, app, lua, font));
        i += 1;
        retain
    });
    std::mem::swap(&mut gui.win.lua_watch, &mut watch_windows);

    // Context menu
    if let Some(menu) = &gui.context_menu {
        let mut close = false;
        egui::Area::new("rootless_ctx_menu".into())
            .fixed_pos(menu.pos)
            .show(ctx, |ui| {
                ui.set_max_width(180.0);
                egui::Frame::menu(ui.style())
                    .inner_margin(2.0)
                    .show(ui, |ui| {
                        if let Some(sel) = app.hex_ui.selection() {
                            ui.separator();
                            if crate::gui::selection_menu::selection_menu(
                                "Selection... ⏷",
                                ui,
                                app,
                                &mut gui.dialogs,
                                &mut gui.msg_dialog,
                                &mut gui.win.regions,
                                sel,
                                &mut gui.fileops,
                            ) {
                                close = true;
                            }
                        }
                        if let Some(view) = menu.data.view {
                            ui.separator();
                            if ui.button("Region properties...").clicked() {
                                gui.win.regions.selected_key = Some(app.region_key_for_view(view));
                                gui.win.regions.open.set(true);
                                close = true;
                            }
                            if ui.button("View properties...").clicked() {
                                gui.win.views.selected = view;
                                gui.win.views.open.set(true);
                                close = true;
                            }
                            ui.menu_button("Change this view to", |ui| {
                                let Some(layout) = app
                                    .meta_state
                                    .meta
                                    .layouts
                                    .get_mut(app.hex_ui.current_layout)
                                else {
                                    return;
                                };
                                for (k, v) in app
                                    .meta_state
                                    .meta
                                    .views
                                    .iter()
                                    .filter(|(k, _)| !layout.contains_view(*k))
                                {
                                    if ui.button(&v.name).clicked() {
                                        layout.change_view_type(view, k);
                                        ui.close_menu();
                                        close = true;
                                        return;
                                    }
                                }
                            });
                            if ui.button("Remove from layout").clicked() {
                                if let Some(layout) = app
                                    .meta_state
                                    .meta
                                    .layouts
                                    .get_mut(app.hex_ui.current_layout)
                                {
                                    layout.remove_view(view);
                                    if app.hex_ui.focused_view == Some(view) {
                                        let first_view =
                                            layout.view_grid.first().and_then(|row| row.first());
                                        app.hex_ui.focused_view = first_view.cloned();
                                    }
                                    close = true;
                                }
                            }
                        }
                        if let Some(byte_off) = menu.data.byte_off {
                            ui.separator();
                            match app
                                .meta_state
                                .meta
                                .bookmarks
                                .iter()
                                .position(|bm| bm.offset == byte_off)
                            {
                                Some(pos) => {
                                    if ui.button("Open bookmark").clicked() {
                                        gui.win.bookmarks.open.set(true);
                                        gui.win.bookmarks.selected = Some(pos);
                                        close = true;
                                    }
                                }
                                None => {
                                    if ui.button("Add bookmark").clicked() {
                                        let bms = &mut app.meta_state.meta.bookmarks;
                                        let idx = bms.len();
                                        bms.push(Bookmark {
                                            offset: byte_off,
                                            label: format!("New @ offset {byte_off}"),
                                            desc: String::new(),
                                            value_type: ValueType::U8(U8),
                                        });
                                        gui.win.bookmarks.open.set(true);
                                        gui.win.bookmarks.selected = Some(idx);
                                        gui.win.bookmarks.edit_name = true;
                                        gui.win.bookmarks.focus_text_edit = true;
                                        close = true;
                                    }
                                }
                            }
                        }
                        ui.separator();
                        if ui.button("Layout properties...").clicked() {
                            gui.win.layouts.open.toggle();
                            close = true;
                        }
                        ui.menu_button("Layouts ->", |ui| {
                            for (key, layout) in app.meta_state.meta.layouts.iter() {
                                if ui.button(&layout.name).clicked() {
                                    App::switch_layout(&mut app.hex_ui, &app.meta_state.meta, key);
                                    ui.close_menu();
                                    close = true;
                                }
                            }
                        });
                    });
            });
        if close {
            gui.context_menu = None;
        }
    }
    // Panels
    let top_re =
        TopBottomPanel::top("top_panel").show(ctx, |ui| top_panel::ui(ui, gui, app, lua, font));
    let bot_re = TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
        bottom_panel::ui(ui, app, mouse_pos, &mut gui.msg_dialog)
    });
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
        let mut win = Window::new(dialog.title());
        let mut open = true;
        if dialog.has_close_button() {
            win = win.open(&mut open)
        }
        win.show(ctx, |ui| {
            retain = dialog.ui(ui, app, gui, lua, font);
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
        font,
    );
    sf_egui.end_frame(rwin)?;
    Ok(true)
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
