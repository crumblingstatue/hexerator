use {
    super::{WinCtx, WindowOpen},
    crate::meta::{VarEntry, VarVal},
    egui::TextBuffer as _,
    egui_extras::Column,
};

#[derive(Default)]
pub struct VarsWindow {
    pub open: WindowOpen,
    pub new_var_name: String,
    pub new_val_val: VarVal = VarVal::U64(0),
}

impl super::Window for VarsWindow {
    fn ui(&mut self, WinCtx { ui, app, .. }: WinCtx) {
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
        ui.group(|ui| {
            ui.label("New");
            ui.horizontal(|ui| {
                ui.label("Name");
                ui.text_edit_singleline(&mut self.new_var_name);
                ui.label("Type");
                let sel_txt = var_val_label(&self.new_val_val);
                egui::ComboBox::new("type_select", "Type").selected_text(sel_txt).show_ui(
                    ui,
                    |ui| {
                        ui.selectable_value(&mut self.new_val_val, VarVal::U64(0), "U64");
                        ui.selectable_value(&mut self.new_val_val, VarVal::I64(0), "I64");
                    },
                );
                if ui.button("Add").clicked() {
                    app.meta_state.meta.vars.insert(
                        self.new_var_name.take(),
                        VarEntry {
                            val: self.new_val_val.clone(),
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
                                VarVal::I64(var) => ui.add(egui::DragValue::new(var)),
                                VarVal::U64(var) => ui.add(egui::DragValue::new(var)),
                            };
                        });
                    });
                }
            });
    }

    fn title(&self) -> &str {
        "Variables"
    }
}

fn var_val_label(var_val: &VarVal) -> &str {
    match var_val {
        VarVal::I64(_) => "i64",
        VarVal::U64(_) => "u64",
    }
}
