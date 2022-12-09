use {
    crate::{app::App, gui::Gui},
    egui::Button,
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
    if ui
        .add(Button::new("Layouts...").shortcut_text("F5"))
        .clicked()
    {
        gui.layouts_window.open.toggle();
        ui.close_menu();
    }
    if ui
        .add(Button::new("Prev view").shortcut_text("Shift+Tab"))
        .clicked()
    {
        app.focus_prev_view_in_layout();
        ui.close_menu();
    }
    if ui
        .add(Button::new("Next view").shortcut_text("Tab"))
        .clicked()
    {
        app.focus_next_view_in_layout();
        ui.close_menu();
    }
    if ui
        .add(Button::new("Views...").shortcut_text("F6"))
        .clicked()
    {
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
