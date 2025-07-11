use {
    crate::{app::App, gui::Gui, hex_ui::Ruler, meta::LayoutMapExt as _},
    constcat::concat,
    egui::{
        Button,
        color_picker::{Alpha, color_picker_color32},
        containers::menu::{MenuConfig, SubMenuButton},
    },
    egui_phosphor::regular as ic,
};

const L_LAYOUT: &str = concat!(ic::LAYOUT, " Layout");
const L_RULER: &str = concat!(ic::RULER, " Ruler");
const L_LAYOUTS: &str = concat!(ic::LAYOUT, " Layouts...");
const L_FOCUS_PREV: &str = concat!(ic::ARROW_FAT_LEFT, " Focus previous");
const L_FOCUS_NEXT: &str = concat!(ic::ARROW_FAT_RIGHT, " Focus next");
const L_VIEWS: &str = concat!(ic::EYE, " Views...");

pub fn ui(ui: &mut egui::Ui, gui: &mut Gui, app: &mut App) {
    if ui.add(Button::new(L_VIEWS).shortcut_text("F6")).clicked() {
        gui.win.views.open.toggle();
    }
    if ui.add(Button::new(L_FOCUS_PREV).shortcut_text("Shift+Tab")).clicked() {
        app.focus_prev_view_in_layout();
    }
    if ui.add(Button::new(L_FOCUS_NEXT).shortcut_text("Tab")).clicked() {
        app.focus_next_view_in_layout();
    }
    ui.menu_button(L_RULER, |ui| match app.focused_view_mut() {
        Some((key, _view)) => match app.hex_ui.rulers.get_mut(&key) {
            Some(ruler) => {
                if ui.button("Remove").clicked() {
                    app.hex_ui.rulers.remove(&key);
                    return;
                }
                ruler.color.with_as_egui_mut(|c| {
                    // Customized color SubMenuButton (taken from the egui demo)
                    let is_bright = c.intensity() > 0.5;
                    let text_color = if is_bright {
                        egui::Color32::BLACK
                    } else {
                        egui::Color32::WHITE
                    };
                    let mut color_button =
                        SubMenuButton::new(egui::RichText::new("Color").color(text_color)).config(
                            MenuConfig::new()
                                .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside),
                        );
                    color_button.button = color_button.button.fill(*c);
                    color_button.ui(ui, |ui| {
                        ui.spacing_mut().slider_width = 200.0;
                        color_picker_color32(ui, c, Alpha::Opaque);
                    });
                });
                ui.label("Frequency");
                ui.add(egui::DragValue::new(&mut ruler.freq));
                ui.label("Horizontal offset");
                ui.add(egui::DragValue::new(&mut ruler.hoffset));
                ui.menu_button("struct", |ui| {
                    for (i, struct_) in app.meta_state.meta.structs.iter().enumerate() {
                        if ui.selectable_label(ruler.struct_idx == Some(i), &struct_.name).clicked()
                        {
                            ruler.struct_idx = Some(i);
                        }
                    }
                    ui.separator();
                    if ui.button("Unassociate").clicked() {
                        ruler.struct_idx = None;
                    }
                });
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
    ui.separator();
    ui.menu_button(L_LAYOUT, |ui| {
        if ui.add(Button::new(L_LAYOUTS).shortcut_text("F5")).clicked() {
            gui.win.layouts.open.toggle();
        }
        if ui.button("➕ Add new").clicked() {
            app.hex_ui.current_layout = app.meta_state.meta.layouts.add_new_default();
            gui.win.layouts.open.set(true);
        }
        ui.separator();
        for (k, v) in &app.meta_state.meta.layouts {
            if ui
                .selectable_label(
                    app.hex_ui.current_layout == k,
                    [ic::LAYOUT, " ", v.name.as_str()].concat(),
                )
                .clicked()
            {
                App::switch_layout(&mut app.hex_ui, &app.meta_state.meta, k);
            }
        }
    });
    ui.checkbox(
        &mut app.preferences.col_change_lock_col,
        "Lock col on col change",
    );
    ui.checkbox(
        &mut app.preferences.col_change_lock_row,
        "Lock row on col change",
    );
}
