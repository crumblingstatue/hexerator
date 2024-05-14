pub use self::{
    about::AboutWindow,
    advanced_open::AdvancedOpenWindow,
    file_diff_result_window::FileDiffResultWindow,
    lua_console::{ConMsg, LuaConsoleWindow},
    lua_help::LuaHelpWindow,
    lua_watch::LuaWatchWindow,
    regions_window::{region_context_menu, RegionsWindow},
    script_manager::ScriptManagerWindow,
    vars::VarsWindow,
};
use {
    self::{
        bookmarks_window::BookmarksWindow, external_command_window::ExternalCommandWindow,
        find_dialog::FindDialog, find_memory_pointers_window::FindMemoryPointersWindow,
        layouts_window::LayoutsWindow, meta_diff_window::MetaDiffWindow,
        open_process_window::OpenProcessWindow, perspectives_window::PerspectivesWindow,
        preferences_window::PreferencesWindow, views_window::ViewsWindow,
    },
    super::Gui,
    crate::app::App,
    egui_sfml::sfml::graphics::{Font, RenderWindow},
};

mod about;
mod advanced_open;
mod bookmarks_window;
pub mod debug_window;
mod external_command_window;
mod file_diff_result_window;
mod find_dialog;
mod find_memory_pointers_window;
mod layouts_window;
mod lua_console;
mod lua_help;
mod lua_watch;
mod meta_diff_window;
mod open_process_window;
mod perspectives_window;
mod preferences_window;
mod regions_window;
mod script_manager;
mod vars;
mod views_window;

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

pub struct WindowCtxt<'a> {
    ui: &'a mut egui::Ui,
    gui: &'a mut crate::gui::Gui,
    app: &'a mut crate::app::App,
    rwin: &'a mut RenderWindow,
    lua: &'a mlua::Lua,
    font: &'a Font,
}

trait Window {
    fn ui(&mut self, ctx: WindowCtxt);
}

impl Windows {
    pub(crate) fn update(
        ctx: &egui::Context,
        gui: &mut Gui,
        app: &mut App,
        rwin: &mut RenderWindow,
        lua: &mlua::Lua,
        font: &Font,
    ) {
        let mut open;
        macro_rules! windows {
            ($($title:expr, $field:ident;)*) => {
                $(
                    let mut win = std::mem::take(&mut gui.win.$field);
                    open = win.open.is();
                    egui::Window::new($title).open(&mut open).show(ctx, |ui| win.ui(WindowCtxt{ ui, gui, app, rwin, lua, font }));
                    if !open {
                        win.open.set(false);
                    }
                    std::mem::swap(&mut gui.win.$field, &mut win);
                )*
            };
        }
        windows! {
            "Find",                    find;
            "Regions",                 regions;
            "Bookmarks",               bookmarks;
            "Layouts",                 layouts;
            "Views",                   views;
            "Variables",               vars;
            "Perspectives",            perspectives;
            "File Diff results",       file_diff_result;
            "Diff against clean meta", meta_diff;
            "Open process",            open_process;
            "Find memory pointers",    find_memory_pointers;
            "Advanced open",           advanced_open;
            "External command",        external_command;
            "Preferences",             preferences;
            "Lua help",                lua_help;
            "Lua console",             lua_console;
            "Script manager",          script_manager;
            "About Hexerator",         about;
        }

        let mut watch_windows = std::mem::take(&mut gui.win.lua_watch);
        let mut i = 0;
        watch_windows.retain_mut(|win| {
            let mut retain = true;
            egui::Window::new(&win.name)
                .id(egui::Id::new("watch_w").with(i))
                .open(&mut retain)
                .show(ctx, |ui| {
                    win.ui(WindowCtxt {
                        ui,
                        gui,
                        app,
                        rwin,
                        lua,
                        font,
                    })
                });
            i += 1;
            retain
        });
        std::mem::swap(&mut gui.win.lua_watch, &mut watch_windows);
    }
}
