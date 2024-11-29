use {
    super::WindowOpen,
    crate::struct_meta_item::{Endian, StructMetaItem, StructTy},
    core::f32,
};

#[derive(Default)]
pub struct StructsWindow {
    pub open: WindowOpen,
    struct_text_buf: String,
    parsed_struct: Option<StructMetaItem>,
    error_label: String,
}

fn read_ty_as_usize_at(data: &[u8], ty: &StructTy, offset: usize) -> Option<usize> {
    ty.read_usize(data.get(offset..)?)
}

impl super::Window for StructsWindow {
    fn ui(&mut self, super::WinCtx { ui, app, .. }: super::WinCtx) {
        let re = ui.add(
            egui::TextEdit::multiline(&mut self.struct_text_buf)
                .code_editor()
                .desired_width(f32::INFINITY)
                .hint_text("Rust struct definition"),
        );
        if re.changed() {
            self.error_label.clear();
            match structparse::Struct::parse(&self.struct_text_buf) {
                Ok(struct_) => match StructMetaItem::new(struct_) {
                    Ok(struct_) => {
                        self.parsed_struct = Some(struct_);
                    }
                    Err(e) => {
                        self.error_label = format!("Resolve error: {e}");
                    }
                },
                Err(e) => {
                    self.error_label = format!("Parse error: {e}");
                }
            }
        }
        egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
            if let Some(struct_) = &mut self.parsed_struct {
                struct_ui(struct_, ui, app);
            }
            if !self.error_label.is_empty() {
                ui.label(egui::RichText::new(&self.error_label).color(egui::Color32::RED));
            }
        });
    }

    fn title(&self) -> &str {
        "Structs"
    }
}

fn struct_ui(struct_: &mut StructMetaItem, ui: &mut egui::Ui, app: &mut crate::app::App) {
    for (off, field) in struct_.fields_with_offsets_mut() {
        ui.horizontal(|ui| {
            if ui.link(off.to_string()).clicked() {
                app.search_focus(off);
            }
            ui.label(format!(
                "{}: {} [size: {}]",
                field.name,
                field.ty,
                field.ty.size()
            ));
            let en = field.ty.endian_mut();
            if ui.checkbox(&mut matches!(en, Endian::Be), en.label()).clicked() {
                en.toggle();
            }
            if ui.button("select").clicked() {
                app.hex_ui.select_a = Some(off);
                app.hex_ui.select_b = Some(off + field.ty.size());
            }
            if let Some(val) = read_ty_as_usize_at(&app.data, &field.ty, off) {
                if ui.link(val.to_string()).on_hover_text("Jump to pointed-to offset").clicked() {
                    app.search_focus(val);
                }
            }
        });
    }
}
