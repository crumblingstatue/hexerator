use {
    crate::{
        app::App,
        gui::{window_open::WindowOpen, Gui},
        meta::{VarEntry, VarVal},
    },
    egui::TextBuffer,
    egui_extras::Column,
};

pub struct VarsWindow {
    pub open: WindowOpen,
    pub new_var_name: String,
    pub new_val_val: VarVal,
}

impl Default for VarsWindow {
    fn default() -> Self {
        Self {
            open: Default::default(),
            new_var_name: Default::default(),
            new_val_val: VarVal::U64(0),
        }
    }
}

impl VarsWindow {
    pub fn ui(ui: &mut egui::Ui, gui: &mut Gui, app: &mut App) {
        ui.style_mut().wrap = Some(false);
        ui.group(|ui| {
            ui.label("New");
            ui.horizontal(|ui| {
                ui.label("Name");
                ui.text_edit_singleline(&mut gui.vars_window.new_var_name);
                ui.label("Type");
                let sel_txt = var_val_label(&gui.vars_window.new_val_val);
                egui::ComboBox::new("type_select", "Type")
                    .selected_text(sel_txt)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut gui.vars_window.new_val_val,
                            VarVal::U64(0),
                            "U64",
                        );
                        ui.selectable_value(
                            &mut gui.vars_window.new_val_val,
                            VarVal::I64(0),
                            "I64",
                        );
                    });
                if ui.button("Add").clicked() {
                    app.meta_state.meta.vars.insert(
                        gui.vars_window.new_var_name.take(),
                        VarEntry {
                            val: gui.vars_window.new_val_val.clone(),
                            desc: String::new(),
                        },
                    );
                }
            });
        });
        egui_extras::TableBuilder::new(ui)
            .columns(Column::auto(), 4)
            .resizable(true)
            .header(32.0, |mut row| {
                row.col(|ui| {
                    ui.label("Name");
                });
                row.col(|ui| {
                    ui.label("Type");
                });
                row.col(|ui| {
                    ui.label("Description");
                });
                row.col(|ui| {
                    ui.label("Value");
                });
            })
            .body(|mut body| {
                for (key, var_ent) in &mut app.meta_state.meta.vars {
                    body.row(32.0, |mut row| {
                        row.col(|ui| {
                            ui.label(key);
                        });
                        row.col(|ui| {
                            ui.label(var_val_label(&var_ent.val));
                        });
                        row.col(|ui| {
                            ui.text_edit_singleline(&mut var_ent.desc);
                        });
                        row.col(|ui| {
                            match &mut var_ent.val {
                                crate::meta::VarVal::I64(var) => ui.add(egui::DragValue::new(var)),
                                crate::meta::VarVal::U64(var) => ui.add(egui::DragValue::new(var)),
                            };
                        });
                    });
                }
            });
    }
}

fn var_val_label(var_val: &VarVal) -> &str {
    match var_val {
        VarVal::I64(_) => "i64",
        VarVal::U64(_) => "u64",
    }
}
