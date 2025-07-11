use {
    super::Gui,
    crate::{app::App, meta::ViewKey, view::ViewportScalar},
    constcat::concat,
    egui_phosphor::regular as ic,
};

const L_SELECTION: &str = concat!(ic::SELECTION, " Selection");
const L_REGION_PROPS: &str = concat!(ic::RULER, " Region properties...");
const L_VIEW_PROPS: &str = concat!(ic::EYE, " View properties...");
const L_CHANGE_THIS_VIEW: &str = concat!(ic::SWAP, " Change this view to");
const L_REMOVE_FROM_LAYOUT: &str = concat!(ic::TRASH, " Remove from layout");
const L_OPEN_BOOKMARK: &str = concat!(ic::BOOKMARK, " Open bookmark");
const L_ADD_BOOKMARK: &str = concat!(ic::BOOKMARK, " Add bookmark");
const L_LAYOUT_PROPS: &str = concat!(ic::LAYOUT, " Layout properties...");
const L_LAYOUTS: &str = concat!(ic::LAYOUT, " Layouts");

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

/// Yoinked from egui source code
fn set_menu_style(style: &mut egui::Style) {
    style.spacing.button_padding = egui::vec2(2.0, 0.0);
    style.visuals.widgets.active.bg_stroke = egui::Stroke::NONE;
    style.visuals.widgets.hovered.bg_stroke = egui::Stroke::NONE;
    style.visuals.widgets.inactive.weak_bg_fill = egui::Color32::TRANSPARENT;
    style.visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
    style.wrap_mode = Some(egui::TextWrapMode::Extend);
}

/// Returns whether to keep root context menu open
#[must_use]
pub(super) fn show(menu: &ContextMenu, ctx: &egui::Context, app: &mut App, gui: &mut Gui) -> bool {
    let mut close = false;
    egui::Area::new("root_ctx_menu".into())
        .kind(egui::UiKind::Menu)
        .order(egui::Order::Foreground)
        .fixed_pos(menu.pos)
        .default_width(ctx.style().spacing.menu_width)
        .sense(egui::Sense::hover())
        .show(ctx, |ui| {
            set_menu_style(ui.style_mut());
            egui::Frame::menu(ui.style()).show(ui, |ui| {
                ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
                    menu_inner_ui(app, ui, gui, &mut close, menu);
                });
            });
        });
    !close
}

fn menu_inner_ui(
    app: &mut App,
    ui: &mut egui::Ui,
    gui: &mut Gui,
    close: &mut bool,
    menu: &ContextMenu,
) {
    if let Some(sel) = app.hex_ui.selection() {
        ui.separator();
        if crate::gui::selection_menu::selection_menu(
            L_SELECTION,
            ui,
            app,
            &mut gui.dialogs,
            &mut gui.msg_dialog,
            &mut gui.win.regions,
            sel,
            &mut gui.fileops,
        ) {
            *close = true;
        }
    }
    if let Some(view) = menu.data.view {
        ui.separator();
        if ui.button(L_REGION_PROPS).clicked() {
            gui.win.regions.selected_key = Some(app.region_key_for_view(view));
            gui.win.regions.open.set(true);
            *close = true;
        }
        if ui.button(L_VIEW_PROPS).clicked() {
            gui.win.views.selected = view;
            gui.win.views.open.set(true);
            *close = true;
        }
        ui.menu_button(L_CHANGE_THIS_VIEW, |ui| {
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
            let Some(layout) = app.meta_state.meta.layouts.get_mut(app.hex_ui.current_layout)
            else {
                return;
            };
            for (k, v) in
                app.meta_state.meta.views.iter().filter(|(k, _)| !layout.contains_view(*k))
            {
                if ui.button(&v.name).clicked() {
                    layout.change_view_type(view, k);

                    *close = true;
                    return;
                }
            }
        });
        if ui.button(L_REMOVE_FROM_LAYOUT).clicked()
            && let Some(layout) = app.meta_state.meta.layouts.get_mut(app.hex_ui.current_layout)
        {
            layout.remove_view(view);
            if app.hex_ui.focused_view == Some(view) {
                let first_view = layout.view_grid.first().and_then(|row| row.first());
                app.hex_ui.focused_view = first_view.cloned();
            }
            *close = true;
        }
    }
    if let Some(byte_off) = menu.data.byte_off {
        ui.separator();
        match app.meta_state.meta.bookmarks.iter().position(|bm| bm.offset == byte_off) {
            Some(pos) => {
                if ui.button(L_OPEN_BOOKMARK).clicked() {
                    gui.win.bookmarks.open.set(true);
                    gui.win.bookmarks.selected = Some(pos);
                    *close = true;
                }
            }
            None => {
                if ui.button(L_ADD_BOOKMARK).clicked() {
                    crate::gui::add_new_bookmark(app, gui, byte_off);
                    *close = true;
                }
            }
        }
    }
    ui.separator();
    if ui.button(L_LAYOUT_PROPS).clicked() {
        gui.win.layouts.open.toggle();
        *close = true;
    }
    ui.menu_button(L_LAYOUTS, |ui| {
        for (key, layout) in app.meta_state.meta.layouts.iter() {
            if ui.button(&layout.name).clicked() {
                App::switch_layout(&mut app.hex_ui, &app.meta_state.meta, key);

                *close = true;
            }
        }
    });
}
