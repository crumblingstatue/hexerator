use {
    super::WindowOpen,
    crate::struct_meta_item::{Endian, IPrimSize, StructMetaItem, StructTy},
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
    ui.separator();
    if let Some([row, _]) = app.row_col_of_cursor()
        && let Some(reg) = app.row_region(row)
    {
        let bm_name = app
            .meta_state
            .meta
            .bookmarks
            .iter()
            .find(|bm| bm.offset == reg.begin)
            .map_or(String::new(), |bm| format!(" ({})", bm.label));
        ui.heading(format!("Object at row {row}{bm_name}"));
        for (off, field) in struct_.fields_with_offsets_mut() {
            ui.horizontal(|ui| {
                let data_off = reg.begin + off;
                ui.label(&field.name);
                let field_bytes_len = field.ty.size();
                let byte_slice = &mut app.data[data_off..data_off + field_bytes_len];
                field_edit_ui(ui, field, byte_slice);
            });
        }
    }
}

fn field_edit_ui(
    ui: &mut egui::Ui,
    field: &crate::struct_meta_item::StructField,
    byte_slice: &mut [u8],
) {
    match &field.ty {
        StructTy::IntegerPrimitive {
            size,
            signed,
            endian,
        } => match (size, signed, endian) {
            (IPrimSize::S8, true, Endian::Le) => {
                ui.add(egui::DragValue::new(
                    &mut bytemuck::cast_slice_mut::<u8, i8>(byte_slice)[0],
                ));
            }
            (IPrimSize::S8, true, Endian::Be) => {
                ui.add(egui::DragValue::new(
                    &mut bytemuck::cast_slice_mut::<u8, i8>(byte_slice)[0],
                ));
            }
            (IPrimSize::S8, false, Endian::Le) => {
                ui.add(egui::DragValue::new(&mut byte_slice[0]));
            }
            (IPrimSize::S8, false, Endian::Be) => {
                ui.add(egui::DragValue::new(&mut byte_slice[0]));
            }
            (IPrimSize::S16, true, Endian::Le) => {
                ui.label("<todo>");
            }
            (IPrimSize::S16, true, Endian::Be) => {
                ui.label("<todo>");
            }
            (IPrimSize::S16, false, Endian::Le) => {
                match bytemuck::try_from_bytes_mut::<u16>(byte_slice) {
                    Ok(num) => {
                        ui.add(egui::DragValue::new(num));
                    }
                    Err(e) => {
                        ui.label(e.to_string());
                    }
                }
            }
            (IPrimSize::S16, false, Endian::Be) => {
                ui.label("<todo>");
            }
            (IPrimSize::S32, true, Endian::Le) => {
                ui.label("<todo>");
            }
            (IPrimSize::S32, true, Endian::Be) => {
                ui.label("<todo>");
            }
            (IPrimSize::S32, false, Endian::Le) => {
                ui.label("<todo>");
            }
            (IPrimSize::S32, false, Endian::Be) => {
                ui.label("<todo>");
            }
            (IPrimSize::S64, true, Endian::Le) => {
                ui.label("<todo>");
            }
            (IPrimSize::S64, true, Endian::Be) => {
                ui.label("<todo>");
            }
            (IPrimSize::S64, false, Endian::Le) => {
                ui.label("<todo>");
            }
            (IPrimSize::S64, false, Endian::Be) => {
                ui.label("<todo>");
            }
        },
        StructTy::Array { .. } => {
            ui.label("<array>");
        }
    }
}
