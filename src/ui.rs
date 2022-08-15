mod bottom_panel;
mod debug_window;
pub mod dialogs;
mod find_dialog;
mod help_window;
pub mod inspect_panel;
mod regions_window;
mod top_menu;
mod top_panel;
mod util;
mod views_window;

use std::fmt::Debug;

use egui_sfml::{
    egui::{self, TopBottomPanel, Window},
    SfEgui,
};
use sfml::graphics::Font;

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
    pub dialogs: Vec<Box<dyn Dialog>>,
    pub views_window: ViewsWindow,
    pub help_window: HelpWindow,
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

use self::{
    find_dialog::FindDialog, help_window::HelpWindow, inspect_panel::InspectPanel,
    regions_window::RegionsWindow, views_window::ViewsWindow,
};

pub fn do_egui(
    sf_egui: &mut SfEgui,
    app: &mut App,
    mouse_pos: ViewportVec,
    window_height: ViewportScalar,
    font: &Font,
) {
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
        open = app.ui.views_window.open;
        Window::new("View configuration")
            .open(&mut open)
            .show(ctx, |ui| ViewsWindow::ui(ui, app, font));
        app.ui.views_window.open = open;
        open = app.ui.help_window.open;
        Window::new("Help")
            .default_size(egui::vec2(800., 600.))
            .open(&mut open)
            .show(ctx, |ui| HelpWindow::ui(ui, app));
        app.ui.help_window.open = open;
        TopBottomPanel::top("top_panel")
            .show(ctx, |ui| top_panel::ui(ui, app, window_height, font));
        TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| bottom_panel::ui(ui, app, mouse_pos));
        egui::SidePanel::right("right_panel").show(ctx, |ui| inspect_panel::ui(ui, app, mouse_pos));
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
