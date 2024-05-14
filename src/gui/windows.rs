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
use self::{
    bookmarks_window::BookmarksWindow, external_command_window::ExternalCommandWindow,
    find_dialog::FindDialog, find_memory_pointers_window::FindMemoryPointersWindow,
    layouts_window::LayoutsWindow, meta_diff_window::MetaDiffWindow,
    open_process_window::OpenProcessWindow, perspectives_window::PerspectivesWindow,
    preferences_window::PreferencesWindow, views_window::ViewsWindow,
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
