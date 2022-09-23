use {
    crate::{
        app::App,
        gui::{util::button_with_shortcut, Gui},
    },
    egui,
};

pub fn ui(ui: &mut egui::Ui, gui: &mut Gui, app: &mut App) {
    ui.menu_button("Layout", |ui| {
        for (k, v) in &app.meta_state.meta.layouts {
            if ui
                .selectable_label(app.hex_ui.current_layout == k, &v.name)
                .clicked()
            {
                App::switch_layout(&mut app.hex_ui, &app.meta_state.meta, k);
                ui.close_menu();
            }
        }
    });
    if button_with_shortcut(ui, "Layouts...", "F5").clicked() {
        gui.layouts_window.open.toggle();
        ui.close_menu();
    }
    if button_with_shortcut(ui, "Prev view", "Shift+Tab").clicked() {
        app.focus_prev_view_in_layout();
        ui.close_menu();
    }
    if button_with_shortcut(ui, "Next view", "Tab").clicked() {
        app.focus_next_view_in_layout();
        ui.close_menu();
    }
    if button_with_shortcut(ui, "Views...", "F6").clicked() {
        gui.views_window.open.toggle();
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
