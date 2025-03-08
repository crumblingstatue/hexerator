use {
    crate::{app::App, gui::Gui, hex_ui::Ruler, meta::LayoutMapExt as _},
    egui::Button,
};

pub fn ui(ui: &mut egui::Ui, gui: &mut Gui, app: &mut App) {
    ui.menu_button("Layout", |ui| {
        for (k, v) in &app.meta_state.meta.layouts {
            if ui.selectable_label(app.hex_ui.current_layout == k, &v.name).clicked() {
                App::switch_layout(&mut app.hex_ui, &app.meta_state.meta, k);
                ui.close_menu();
            }
        }
        ui.separator();
        if ui.button("➕ Add new layout").clicked() {
            app.hex_ui.current_layout = app.meta_state.meta.layouts.add_new_default();
            gui.win.layouts.open.set(true);
            ui.close_menu();
        }
    });
    ui.menu_button("Ruler", |ui| match app.focused_view_mut() {
        Some((key, _view)) => match app.hex_ui.rulers.get_mut(&key) {
            Some(ruler) => {
                if ui.button("Remove").clicked() {
                    app.hex_ui.rulers.remove(&key);
                    return;
                }
                ui.label("Color (right or middle click when open)");
                ruler.color.with_as_egui_mut(|c| {
                    ui.color_edit_button_srgba(c);
                });
                ui.label("Frequency");
                ui.add(egui::DragValue::new(&mut ruler.freq));
                ui.label("Horizontal offset");
                ui.add(egui::DragValue::new(&mut ruler.hoffset));
            }
            None => {
                if ui.button("Add ruler for current view").clicked() {
                    app.hex_ui.rulers.insert(key, Ruler::default());
                }
            }
        },
        None => {
            ui.label("<No active view>");
        }
    });
    if ui.add(Button::new("Layouts...").shortcut_text("F5")).clicked() {
        gui.win.layouts.open.toggle();
        ui.close_menu();
    }
    if ui.add(Button::new("Prev view").shortcut_text("Shift+Tab")).clicked() {
        app.focus_prev_view_in_layout();
        ui.close_menu();
    }
    if ui.add(Button::new("Next view").shortcut_text("Tab")).clicked() {
        app.focus_next_view_in_layout();
        ui.close_menu();
    }
    if ui.add(Button::new("Views...").shortcut_text("F6")).clicked() {
        gui.win.views.open.toggle();
        ui.close_menu();
    }
    ui.checkbox(
        &mut app.preferences.col_change_lock_col,
        "Lock col on col change",
    );
    ui.checkbox(
        &mut app.preferences.col_change_lock_row,
        "Lock row on col change",
    );
}
