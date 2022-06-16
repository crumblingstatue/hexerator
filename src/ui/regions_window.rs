use egui_sfml::egui::{self, Ui};

use crate::app::{App, NamedRegion};

#[derive(Debug, Default)]
pub struct RegionsWindow {
    pub open: bool,
    pub rename_idx: Option<usize>,
}

impl RegionsWindow {
    pub fn ui(ui: &mut Ui, app: &mut App) {
        let button = egui::Button::new("Add selection as region");
        match app.selection {
            Some(sel) => {
                if ui.add(button).clicked() {
                    app.regions.push(NamedRegion {
                        name: String::from("<Unnamed>"),
                        region: sel,
                    })
                }
            }
            None => {
                ui.add_enabled(false, button);
            }
        }
        ui.separator();
        let mut idx = 0;
        app.regions.retain_mut(|region| {
            let mut retain = true;
            ui.horizontal(|ui| {
                if app.ui.regions_window.rename_idx == Some(idx) {
                    if ui.text_edit_singleline(&mut region.name).lost_focus() {
                        app.ui.regions_window.rename_idx = None;
                    }
                } else {
                    let re = ui.button(&region.name);
                    if re.double_clicked() {
                        app.ui.regions_window.rename_idx = Some(idx);
                    } else if re.clicked() {
                        app.selection = Some(region.region);
                    }
                }
                ui.label(format!("{}..={}", region.region.begin, region.region.end));
                if ui.button("🗑").clicked() {
                    retain = false;
                }
            });
            idx += 1;
            retain
        });
    }
}
