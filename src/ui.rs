mod bookmarks_window;
mod bottom_panel;
mod debug_window;
pub mod dialogs;
mod file_diff_result_window;
mod find_dialog;
mod help_window;
pub mod inspect_panel;
mod layouts_window;
mod meta_diff_window;
mod perspectives_window;
mod regions_window;
mod top_menu;
mod top_panel;
mod util;
mod views_window;
mod window_open;

use std::fmt::Debug;

use egui_sfml::sfml::graphics::Font;
use egui_sfml::{
    egui::{self, TopBottomPanel, Window},
    SfEgui,
};

use crate::meta::ViewKey;
use crate::{
    app::App,
    view::{ViewportScalar, ViewportVec},
};

#[derive(Default)]
pub struct Ui {
    pub inspect_panel: InspectPanel,
    pub find_dialog: FindDialog,
    pub center_offset_input: String,
    pub seek_byte_offset_input: String,
    pub regions_window: RegionsWindow,
    pub bookmarks_window: BookmarksWindow,
    pub dialogs: Vec<Box<dyn Dialog>>,
    pub layouts_window: LayoutsWindow,
    pub views_window: ViewsWindow,
    pub perspectives_window: PerspectivesWindow,
    pub help_window: HelpWindow,
    pub file_diff_result_window: FileDiffResultWindow,
    pub context_menu: Option<ContextMenu>,
    pub meta_diff_window: MetaDiffWindow,
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

pub enum ContextMenuData {
    ViewByte { view: ViewKey, byte_off: usize },
}

pub trait Dialog: Debug {
    fn title(&self) -> &str;
    /// Do the ui for this dialog. Returns whether to keep this dialog open.
    fn ui(&mut self, ui: &mut egui::Ui, app: &mut App) -> bool;
}

impl Ui {
    pub fn add_dialog<D: Dialog + 'static>(&mut self, dialog: D) {
        self.dialogs.push(Box::new(dialog));
    }
}

use self::bookmarks_window::BookmarksWindow;
use self::file_diff_result_window::FileDiffResultWindow;
use self::layouts_window::LayoutsWindow;
use self::meta_diff_window::MetaDiffWindow;
use self::{
    find_dialog::FindDialog, help_window::HelpWindow, inspect_panel::InspectPanel,
    perspectives_window::PerspectivesWindow, regions_window::RegionsWindow,
    views_window::ViewsWindow,
};

pub fn do_egui(sf_egui: &mut SfEgui, app: &mut App, mouse_pos: ViewportVec, font: &Font) {
    sf_egui.do_frame(|ctx| {
        let mut open = gamedebug_core::enabled();
        let was_open = open;
        Window::new("Debug")
            .open(&mut open)
            .show(ctx, debug_window::ui);
        if was_open && !open {
            gamedebug_core::toggle();
        }
        open = app.ui.find_dialog.open;
        Window::new("Find")
            .open(&mut open)
            .show(ctx, |ui| FindDialog::ui(ui, app));
        app.ui.find_dialog.open = open;
        open = app.ui.regions_window.open;
        Window::new("Regions")
            .open(&mut open)
            .show(ctx, |ui| RegionsWindow::ui(ui, app));
        app.ui.regions_window.open = open;
        open = app.ui.bookmarks_window.open.is_open();
        Window::new("Bookmarks")
            .open(&mut open)
            .show(ctx, |ui| BookmarksWindow::ui(ui, app));
        app.ui.bookmarks_window.open.set_open(open);
        open = app.ui.layouts_window.open.is_open();
        Window::new("Layouts")
            .open(&mut open)
            .show(ctx, |ui| LayoutsWindow::ui(ui, app));
        app.ui.layouts_window.open.set_open(open);
        open = app.ui.views_window.open.is_open();
        Window::new("Views")
            .open(&mut open)
            .show(ctx, |ui| ViewsWindow::ui(ui, app, font));
        app.ui.views_window.open.set_open(open);
        open = app.ui.perspectives_window.open.is_open();
        Window::new("Perspectives")
            .open(&mut open)
            .show(ctx, |ui| PerspectivesWindow::ui(ui, app));
        app.ui.perspectives_window.open.set_open(open);
        open = app.ui.help_window.open;
        Window::new("Help")
            .default_size(egui::vec2(800., 600.))
            .open(&mut open)
            .show(ctx, |ui| HelpWindow::ui(ui, app));
        app.ui.help_window.open = open;
        open = app.ui.file_diff_result_window.open.is_open();
        Window::new("File diff results")
            .open(&mut open)
            .show(ctx, |ui| FileDiffResultWindow::ui(ui, app));
        app.ui.file_diff_result_window.open.set_open(open);
        open = app.ui.meta_diff_window.open.is_open();
        Window::new("Diff against clean meta")
            .open(&mut open)
            .show(ctx, |ui| MetaDiffWindow::ui(ui, app));
        app.ui.meta_diff_window.open.set_open(open);
        // Context menu
        if let Some(menu) = &app.ui.context_menu {
            let mut close = false;
            egui::Area::new("rootless_ctx_menu")
                .fixed_pos(menu.pos)
                .show(ctx, |ui| {
                    ui.set_max_width(180.0);
                    egui::Frame::menu(ui.style())
                        .inner_margin(2.0)
                        .show(ui, |ui| match &menu.data {
                            &ContextMenuData::ViewByte { view, byte_off } => {
                                if ui
                                    .button("Increase byte")
                                    .on_hover_text("Context menu test")
                                    .clicked()
                                {
                                    app.data[byte_off] += 1;
                                    close = true;
                                }
                                ui.separator();
                                if ui.button("View properties...").clicked() {
                                    app.ui.views_window.selected = view;
                                    app.ui.views_window.open.set_open(true);
                                    close = true;
                                }
                            }
                        });
                });
            if close {
                app.ui.context_menu = None;
            }
        }
        // Panels
        let top_re = TopBottomPanel::top("top_panel").show(ctx, |ui| top_panel::ui(ui, app, font));
        let bot_re = TopBottomPanel::bottom("bottom_panel")
            .show(ctx, |ui| bottom_panel::ui(ui, app, mouse_pos));
        let right_re = egui::SidePanel::right("right_panel")
            .show(ctx, |ui| inspect_panel::ui(ui, app, mouse_pos))
            .response;
        let padding = 2;
        app.hex_iface_rect.x = padding;
        #[expect(
            clippy::cast_possible_truncation,
            reason = "Window size can't exceed i16"
        )]
        {
            app.hex_iface_rect.y = top_re.response.rect.bottom() as ViewportScalar + padding;
        }
        #[expect(
            clippy::cast_possible_truncation,
            reason = "Window size can't exceed i16"
        )]
        {
            app.hex_iface_rect.w = right_re.rect.left() as ViewportScalar - padding * 2;
        }
        #[expect(
            clippy::cast_possible_truncation,
            reason = "Window size can't exceed i16"
        )]
        {
            app.hex_iface_rect.h =
                (bot_re.response.rect.top() as ViewportScalar - app.hex_iface_rect.y) - padding * 2;
        }
        let mut dialogs: Vec<_> = std::mem::take(&mut app.ui.dialogs);
        dialogs.retain_mut(|dialog| {
            let mut retain = true;
            Window::new(dialog.title()).show(ctx, |ui| {
                retain = dialog.ui(ui, app);
            });
            retain
        });
        app.ui.dialogs = dialogs;
    });
}
