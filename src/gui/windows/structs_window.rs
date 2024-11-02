use {super::WindowOpen, crate::struct_meta_item::StructMetaItem, core::f32};

#[derive(Default)]
pub struct StructsWindow {
    pub open: WindowOpen,
    struct_text_buf: String,
}

impl super::Window for StructsWindow {
    fn ui(&mut self, super::WinCtx { ui, app, .. }: super::WinCtx) {
        ui.add(
            egui::TextEdit::multiline(&mut self.struct_text_buf)
                .desired_width(f32::INFINITY)
                .hint_text("Rust struct definition"),
        );
        egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
            match structparse::Struct::parse(&self.struct_text_buf) {
                Ok(struct_) => match StructMetaItem::new(struct_) {
                    Ok(struct_) => {
                        for (off, field) in struct_.fields_with_offsets() {
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
                                if ui.button("select").clicked() {
                                    app.hex_ui.select_a = Some(off);
                                    app.hex_ui.select_b = Some(off + field.ty.size());
                                }
                            });
                        }
                    }
                    Err(e) => {
                        ui.label(format!("Resolve error: {e}"));
                    }
                },
                Err(e) => {
                    ui.label(format!("Parse error: {e}"));
                }
            }
        });
    }

    fn title(&self) -> &str {
        "Structs"
    }
}
