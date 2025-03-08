use {
    super::WindowOpen,
    crate::struct_meta_item::{Endian, IPrimSize, StructMetaItem, StructTy},
    egui_code_editor::{CodeEditor, Syntax},
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
        let re = CodeEditor::default()
            .with_syntax(Syntax::rust())
            .show(ui, &mut self.struct_text_buf)
            .response;

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

trait ToFromBytes: Sized {
    const LEN: usize = std::mem::size_of::<Self>();
    fn from_bytes(bytes: [u8; Self::LEN], endian: Endian) -> Self;
    fn to_bytes(&self, endian: Endian) -> [u8; Self::LEN];
}

fn with_bytes_as_primitive<T, F>(bytes: &mut [u8], endian: Endian, mut fun: F)
where
    T: ToFromBytes,
    F: FnMut(&mut T),
    [(); T::LEN]:,
{
    if let Ok(arr) = bytes.try_into() {
        let mut prim = T::from_bytes(arr, endian);
        fun(&mut prim);
        bytes.copy_from_slice(prim.to_bytes(endian).as_slice());
    }
}

macro_rules! to_from_impl {
    ($prim:ty) => {
        impl ToFromBytes for $prim {
            fn from_bytes(bytes: [u8; Self::LEN], endian: Endian) -> Self {
                match endian {
                    Endian::Le => <$prim>::from_le_bytes(bytes),
                    Endian::Be => <$prim>::from_be_bytes(bytes),
                }
            }
            fn to_bytes(&self, endian: Endian) -> [u8; Self::LEN] {
                match endian {
                    Endian::Le => self.to_le_bytes(),
                    Endian::Be => self.to_be_bytes(),
                }
            }
        }
    };
}

to_from_impl!(i8);
to_from_impl!(u8);
to_from_impl!(i16);
to_from_impl!(u16);
to_from_impl!(i32);
to_from_impl!(u32);
to_from_impl!(i64);
to_from_impl!(u64);
to_from_impl!(f32);
to_from_impl!(f64);

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
            (IPrimSize::S8, true, _) => {
                with_bytes_as_primitive(byte_slice, *endian, |num: &mut i8| {
                    ui.add(egui::DragValue::new(num));
                });
            }
            (IPrimSize::S8, false, _) => {
                with_bytes_as_primitive(byte_slice, *endian, |num: &mut u8| {
                    ui.add(egui::DragValue::new(num));
                });
            }
            (IPrimSize::S16, true, _) => {
                with_bytes_as_primitive(byte_slice, *endian, |num: &mut i16| {
                    ui.add(egui::DragValue::new(num));
                });
            }
            (IPrimSize::S16, false, _) => {
                with_bytes_as_primitive(byte_slice, *endian, |num: &mut u16| {
                    ui.add(egui::DragValue::new(num));
                });
            }
            (IPrimSize::S32, true, _) => {
                with_bytes_as_primitive(byte_slice, *endian, |num: &mut i32| {
                    ui.add(egui::DragValue::new(num));
                });
            }
            (IPrimSize::S32, false, _) => {
                with_bytes_as_primitive(byte_slice, *endian, |num: &mut u32| {
                    ui.add(egui::DragValue::new(num));
                });
            }
            (IPrimSize::S64, true, _) => {
                with_bytes_as_primitive(byte_slice, *endian, |num: &mut i64| {
                    ui.add(egui::DragValue::new(num));
                });
            }
            (IPrimSize::S64, false, _) => {
                with_bytes_as_primitive(byte_slice, *endian, |num: &mut u64| {
                    ui.add(egui::DragValue::new(num));
                });
            }
        },
        StructTy::FloatPrimitive { size, endian } => match size {
            IPrimSize::S8 => todo!(),
            IPrimSize::S16 => todo!(),
            IPrimSize::S32 => {
                with_bytes_as_primitive(byte_slice, *endian, |num: &mut f32| {
                    ui.add(egui::DragValue::new(num));
                });
            }
            IPrimSize::S64 => {
                with_bytes_as_primitive(byte_slice, *endian, |num: &mut f64| {
                    ui.add(egui::DragValue::new(num));
                });
            }
        },
        StructTy::Array { .. } => {
            ui.label("<array>");
        }
    }
}
