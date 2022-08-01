use egui_sfml::egui::{self, DragValue, Ui};

use crate::app::{App, NamedRegion};

#[derive(Debug, Default)]
pub struct RegionsWindow {
    pub open: bool,
    pub status: Status,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Status {
    Init,
    Rename(usize),
    EditBegin(usize),
    EditEnd(usize),
}

impl Default for Status {
    fn default() -> Self {
        Self::Init
    }
}

impl RegionsWindow {
    pub fn ui(ui: &mut Ui, app: &mut App) {
        enum Action {
            SetCursor(usize),
        }
        let button = egui::Button::new("Add selection as region");
        match App::selection(&app.select_a, &app.select_b) {
            Some(sel) => {
                if ui.add(button).clicked() {
                    app.regions.push(NamedRegion {
                        name: String::from("<Unnamed>"),
                        region: sel,
                    });
                    app.meta_dirty = true;
                }
            }
            None => {
                ui.add_enabled(false, button);
            }
        }
        ui.separator();
        let mut idx = 0;
        let mut action = None;
        app.regions.retain_mut(|region| {
            let mut retain = true;
            ui.horizontal(|ui| {
                if app.ui.regions_window.status == Status::Rename(idx) {
                    if ui.text_edit_singleline(&mut region.name).lost_focus() {
                        app.ui.regions_window.status = Status::Init;
                    }
                } else {
                    let re = ui.button(&region.name);
                    if re.double_clicked() {
                        app.ui.regions_window.status = Status::Rename(idx);
                    } else if re.clicked() {
                        App::set_selection(&mut app.select_a, &mut app.select_b, region.region);
                    }
                }
                if app.ui.regions_window.status == Status::EditBegin(idx) {
                    if ui
                        .add(DragValue::new(&mut region.region.begin))
                        .lost_focus()
                    {
                        app.ui.regions_window.status = Status::Init;
                    }
                } else {
                    let re = ui.button(region.region.begin.to_string());
                    if re.double_clicked() {
                        app.ui.regions_window.status = Status::EditBegin(idx);
                    } else if re.clicked() {
                        action = Some(Action::SetCursor(region.region.begin));
                    }
                }
                ui.label("..=");
                if app.ui.regions_window.status == Status::EditEnd(idx) {
                    if ui.add(DragValue::new(&mut region.region.end)).lost_focus() {
                        app.ui.regions_window.status = Status::Init;
                    }
                } else {
                    let re = ui.button(region.region.end.to_string());
                    if re.double_clicked() {
                        app.ui.regions_window.status = Status::EditEnd(idx);
                    } else if re.clicked() {
                        action = Some(Action::SetCursor(region.region.end));
                    }
                }
                ui.label(format!("Size: {}", region.region.len()));
                if ui.button("🗑").clicked() {
                    retain = false;
                    app.meta_dirty = true;
                }
            });
            idx += 1;
            retain
        });
        if let Some(action) = action {
            match action {
                Action::SetCursor(offset) => {
                    app.edit_state.cursor = offset;
                    app.center_view_on_offset(offset);
                }
            }
        }
    }
}
