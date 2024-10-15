use {
    super::Gui,
    crate::{
        app::App,
        meta::{
            value_type::{self, ValueType},
            Bookmark, ViewKey,
        },
        view::ViewportScalar,
    },
};

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

/// Returns whether to keep root context menu open
#[must_use]
pub(super) fn show(menu: &ContextMenu, ctx: &egui::Context, app: &mut App, gui: &mut Gui) -> bool {
    let mut close = false;
    egui::Area::new("root_ctx_menu".into()).fixed_pos(menu.pos).show(ctx, |ui| {
        ui.set_max_width(180.0);
        egui::Frame::menu(ui.style()).inner_margin(2.0).show(ui, |ui| {
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
                    let Some(layout) =
                        app.meta_state.meta.layouts.get_mut(app.hex_ui.current_layout)
                    else {
                        return;
                    };
                    for (k, v) in
                        app.meta_state.meta.views.iter().filter(|(k, _)| !layout.contains_view(*k))
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
                    if let Some(layout) =
                        app.meta_state.meta.layouts.get_mut(app.hex_ui.current_layout)
                    {
                        layout.remove_view(view);
                        if app.hex_ui.focused_view == Some(view) {
                            let first_view = layout.view_grid.first().and_then(|row| row.first());
                            app.hex_ui.focused_view = first_view.cloned();
                        }
                        close = true;
                    }
                }
            }
            if let Some(byte_off) = menu.data.byte_off {
                ui.separator();
                match app.meta_state.meta.bookmarks.iter().position(|bm| bm.offset == byte_off) {
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
                                value_type: ValueType::U8(value_type::U8),
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
    !close
}
