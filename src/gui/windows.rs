pub use self::{
    file_diff_result::FileDiffResultWindow,
    lua_console::{ConMsg, LuaConsoleWindow},
    regions::{RegionsWindow, region_context_menu},
};
use {
    self::{
        about::AboutWindow, bookmarks::BookmarksWindow, external_command::ExternalCommandWindow,
        find_dialog::FindDialog, find_memory_pointers::FindMemoryPointersWindow,
        layouts::LayoutsWindow, lua_help::LuaHelpWindow, lua_watch::LuaWatchWindow,
        meta_diff::MetaDiffWindow, open_process::OpenProcessWindow,
        perspectives::PerspectivesWindow, preferences::PreferencesWindow,
        script_manager::ScriptManagerWindow, structs::StructsWindow, vars::VarsWindow,
        views::ViewsWindow, zero_partition::ZeroPartition,
    },
    super::Gui,
    crate::app::App,
    egui_sfml::sfml::graphics::Font,
    lua_editor::LuaEditorWindow,
};

mod about;
mod bookmarks;
pub mod debug;
mod external_command;
mod file_diff_result;
mod find_dialog;
mod find_memory_pointers;
mod layouts;
mod lua_console;
mod lua_editor;
mod lua_help;
mod lua_watch;
mod meta_diff;
mod open_process;
mod perspectives;
mod preferences;
mod regions;
mod script_manager;
mod structs;
mod vars;
mod views;
mod zero_partition;

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
    pub external_command: ExternalCommandWindow,
    pub preferences: PreferencesWindow,
    pub about: AboutWindow,
    pub vars: VarsWindow,
    pub lua_editor: LuaEditorWindow,
    pub lua_help: LuaHelpWindow,
    pub lua_console: LuaConsoleWindow,
    pub lua_watch: Vec<LuaWatchWindow>,
    pub script_manager: ScriptManagerWindow,
    pub meta_diff: MetaDiffWindow,
    pub zero_partition: ZeroPartition,
    pub structs: StructsWindow,
}

#[derive(Default)]
pub(crate) struct WindowOpen {
    open: bool,
    just_opened: bool,
}

impl WindowOpen {
    /// Open if closed, close if opened
    pub fn toggle(&mut self) {
        self.open ^= true;
        if self.open {
            self.just_opened = true;
        }
    }
    /// Wheter the window is open
    fn is(&self) -> bool {
        self.open
    }
    /// Set whether the window is open
    pub fn set(&mut self, open: bool) {
        if !self.open && open {
            self.just_opened = true;
        }
        self.open = open;
    }
    /// Whether the window was opened just now (this frame)
    fn just_now(&self) -> bool {
        self.just_opened
    }
}

struct WinCtx<'a> {
    ui: &'a mut egui::Ui,
    gui: &'a mut Gui,
    app: &'a mut App,
    lua: &'a mlua::Lua,
    font_size: u16,
    line_spacing: u16,
    font: &'a Font,
}

trait Window {
    fn ui(&mut self, ctx: WinCtx);
    fn title(&self) -> &str;
}

impl Windows {
    pub(crate) fn update(
        ctx: &egui::Context,
        gui: &mut Gui,
        app: &mut App,
        lua: &mlua::Lua,
        font_size: u16,
        line_spacing: u16,
        font: &Font,
    ) {
        let mut open;
        macro_rules! windows {
            ($($field:ident,)*) => {
                $(
                    let mut win = std::mem::take(&mut gui.win.$field);
                    open = win.open.is();
                    egui::Window::new(win.title()).open(&mut open).show(ctx, |ui| win.ui(WinCtx{ ui, gui, app, lua, font_size, line_spacing, font }));
                    win.open.just_opened = false;
                    if !open {
                        win.open.set(false);
                    }
                    std::mem::swap(&mut gui.win.$field, &mut win);
                )*
            };
        }
        windows!(
            find,
            regions,
            bookmarks,
            layouts,
            views,
            vars,
            perspectives,
            file_diff_result,
            meta_diff,
            open_process,
            find_memory_pointers,
            external_command,
            preferences,
            lua_editor,
            lua_help,
            lua_console,
            script_manager,
            about,
            zero_partition,
            structs,
        );

        let mut watch_windows = std::mem::take(&mut gui.win.lua_watch);
        let mut i = 0;
        watch_windows.retain_mut(|win| {
            let mut retain = true;
            egui::Window::new(&win.name)
                .id(egui::Id::new("watch_w").with(i))
                .open(&mut retain)
                .show(ctx, |ui| {
                    win.ui(WinCtx {
                        ui,
                        gui,
                        app,
                        lua,
                        font_size,
                        line_spacing,
                        font,
                    });
                });
            i += 1;
            retain
        });
        std::mem::swap(&mut gui.win.lua_watch, &mut watch_windows);
    }
    pub fn add_lua_watch_window(&mut self) {
        self.lua_watch.push(LuaWatchWindow::default());
    }
}
