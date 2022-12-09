use {
    crate::{
        app::{col_change_impl_view_perspective, App},
        gui::Gui,
    },
    egui::Button,
};

pub fn ui(ui: &mut egui::Ui, gui: &mut Gui, app: &mut App) {
    if ui
        .add(Button::new("Perspectives...").shortcut_text("F7"))
        .clicked()
    {
        gui.perspectives_window.open.toggle();
        ui.close_menu();
    }
    let Some(view_key) = app.hex_ui.focused_view else { return };
    let view = &mut app.meta_state.meta.views[view_key].view;
    if ui.button("Set offset to cursor").clicked() {
        app.meta_state.meta.low.regions
            [app.meta_state.meta.low.perspectives[view.perspective].region]
            .region
            .begin = app.edit_state.cursor;
        ui.close_menu();
    }
    if ui
        .button("Fill focused view")
        .on_hover_text("Make the column count as big as the active view can fit")
        .clicked()
    {
        ui.close_menu();
        view.scroll_offset.pix_xoff = 0;
        view.scroll_offset.col = 0;
        #[expect(clippy::cast_sign_loss, reason = "columns is never negative")]
        {
            let cols = view.cols() as usize;
            col_change_impl_view_perspective(
                view,
                &mut app.meta_state.meta.low.perspectives,
                &app.meta_state.meta.low.regions,
                |c| *c = cols,
                app.preferences.col_change_lock_col,
                app.preferences.col_change_lock_row,
            );
        }
    }
    if ui
        .checkbox(
            &mut app.meta_state.meta.low.perspectives[view.perspective].flip_row_order,
            "Flip row order (experimental)",
        )
        .clicked()
    {
        ui.close_menu();
    }
}
