use {
    super::WindowOpen,
    crate::{
        meta::Meta,
        struct_meta_item::{Endian, StructMetaItem, StructPrimitive, StructTy},
    },
    egui_code_editor::{CodeEditor, Syntax},
};

#[derive(Default)]
pub struct StructsWindow {
    pub open: WindowOpen,
    struct_text_buf: String,
    parsed_struct: Option<StructMetaItem>,
    error_label: String,
    selected_idx: usize,
    tab: Tab = Tab::Fields,
}

#[derive(PartialEq)]
enum Tab {
    Fields,
    AtRow,
}

impl super::Window for StructsWindow {
    fn ui(&mut self, super::WinCtx { ui, app, .. }: super::WinCtx) {
        if self.open.just_now()
            && let Some(struct_) = app.meta_state.meta.structs.get(self.selected_idx)
        {
            self.struct_text_buf = struct_.src.clone();
            self.parsed_struct = Some(struct_.clone());
        }
        let top_h = ui.available_height() - 32.0;
        ui.horizontal(|ui| {
            ui.set_max_height(top_h);
            ui.vertical(|ui| {
                self.picker_ui(&app.meta_state.meta, ui);
            });
            ui.separator();
            self.editor_ui(ui);
            ui.separator();
            ui.vertical(|ui| {
                self.parsed_struct_ui(ui, app);
            });
        });
        ui.separator();
        self.bottom_bar_ui(ui, app);
    }

    fn title(&self) -> &str {
        "Structs"
    }
}

impl StructsWindow {
    fn refresh(&mut self, meta: &Meta) {
        self.struct_text_buf.clear();
        self.parsed_struct = None;
        if let Some(struct_) = meta.structs.get(self.selected_idx) {
            self.struct_text_buf = struct_.src.clone();
            self.parsed_struct = Some(struct_.clone());
        }
    }
    fn picker_ui(&mut self, meta: &Meta, ui: &mut egui::Ui) {
        for (i, struct_) in meta.structs.iter().enumerate() {
            if ui.selectable_label(self.selected_idx == i, &struct_.name).clicked() {
                self.selected_idx = i;
                self.struct_text_buf = struct_.src.clone();
                self.parsed_struct = Some(struct_.clone());
            }
        }
    }
    fn editor_ui(&mut self, ui: &mut egui::Ui) {
        let re = CodeEditor::default()
            .with_syntax(Syntax::rust())
            .desired_width(300.0)
            .show(ui, &mut self.struct_text_buf)
            .response;

        if re.changed() {
            self.error_label.clear();
            match structparse::Struct::parse(&self.struct_text_buf) {
                Ok(struct_) => match StructMetaItem::new(struct_, self.struct_text_buf.clone()) {
                    Ok(struct_) => {
                        self.parsed_struct = Some(struct_);
                    }
                    Err(e) => {
                        self.error_label = format!("Resolve error: {e}");
                    }
                },
                Err(e) => {
                    self.parsed_struct = None;
                    self.error_label = format!("Parse error: {e}");
                }
            }
        }
    }
    fn parsed_struct_ui(&mut self, ui: &mut egui::Ui, app: &mut crate::app::App) {
        egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            if let Some(struct_) = &mut self.parsed_struct {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.tab, Tab::Fields, "Fields");
                    if let Some([row, _col]) = app.row_col_of_cursor()
                        && let Some(reg) = app.row_region(row)
                    {
                        let bm_name = app
                            .meta_state
                            .meta
                            .bookmarks
                            .iter()
                            .find(|bm| bm.offset == reg.begin)
                            .map_or(String::new(), |bm| format!(" ({})", bm.label));
                        ui.selectable_value(
                            &mut self.tab,
                            Tab::AtRow,
                            format!("At row {row}{bm_name}"),
                        );
                    }
                });
                ui.separator();
                match self.tab {
                    Tab::Fields => fields_ui(struct_, ui),
                    Tab::AtRow => at_row_ui(struct_, ui, app),
                }
            }
            if !self.error_label.is_empty() {
                let label = egui::Label::new(
                    egui::RichText::new(&self.error_label).color(egui::Color32::RED),
                )
                .extend();
                ui.add(label);
            }
        });
    }
    fn bottom_bar_ui(&mut self, ui: &mut egui::Ui, app: &mut crate::app::App) {
        match &mut self.parsed_struct {
            Some(struct_) => {
                let mut del = false;
                let mut refresh = false;
                ui.horizontal(|ui| {
                    if ui.button("Save").clicked() {
                        struct_.src = self.struct_text_buf.clone();
                        if let Some(s) =
                            app.meta_state.meta.structs.iter_mut().find(|s| s.name == struct_.name)
                        {
                            *s = struct_.clone();
                        } else {
                            app.meta_state.meta.structs.push(struct_.clone());
                        }
                    }
                    if ui.button("Delete").clicked() {
                        del = true;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Restore all").clicked() {
                            app.meta_state.meta.structs = app.meta_state.clean_meta.structs.clone();
                            refresh = true;
                        }
                    });
                });
                if del {
                    if self.selected_idx < app.meta_state.meta.structs.len() {
                        app.meta_state.meta.structs.remove(self.selected_idx);
                    }
                    self.selected_idx = self.selected_idx.saturating_sub(1);
                    self.refresh(&app.meta_state.meta);
                }
                if refresh {
                    self.refresh(&app.meta_state.meta);
                }
            }
            None => {
                ui.add_enabled(false, egui::Button::new("Save"));
            }
        }
    }
}

fn fields_ui(struct_: &mut StructMetaItem, ui: &mut egui::Ui) {
    for (_off, field) in struct_.fields_with_offsets_mut() {
        ui.horizontal(|ui| {
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
        });
    }
}

fn at_row_ui(struct_: &mut StructMetaItem, ui: &mut egui::Ui, app: &mut crate::app::App) {
    if let Some([row, _]) = app.row_col_of_cursor()
        && let Some(reg) = app.row_region(row)
    {
        for (off, field) in struct_.fields_with_offsets_mut() {
            ui.horizontal(|ui| {
                let data_off = reg.begin + off;
                if ui.link(off.to_string()).clicked() {
                    app.search_focus(data_off);
                }
                ui.label(&field.name);
                let field_bytes_len = field.ty.size();
                if let Some(byte_slice) = app.data.get_mut(data_off..data_off + field_bytes_len) {
                    field_edit_ui(ui, field, byte_slice);
                } else {
                    ui.label("<out of bounds>");
                }
                if ui.button("select").clicked() {
                    app.hex_ui.select_a = Some(data_off);
                    app.hex_ui.select_b = Some(data_off + field.ty.size().saturating_sub(1));
                }
            });
        }
    }
}

trait ToFromBytes: Sized {
    const LEN: usize = size_of::<Self>();
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
        StructTy::Primitive { ty, endian } => match ty {
            StructPrimitive::I8 => {
                with_bytes_as_primitive(byte_slice, *endian, |num: &mut i8| {
                    ui.add(egui::DragValue::new(num));
                });
            }
            StructPrimitive::U8 => {
                with_bytes_as_primitive(byte_slice, *endian, |num: &mut u8| {
                    ui.add(egui::DragValue::new(num));
                });
            }
            StructPrimitive::I16 => {
                with_bytes_as_primitive(byte_slice, *endian, |num: &mut i16| {
                    ui.add(egui::DragValue::new(num));
                });
            }
            StructPrimitive::U16 => {
                with_bytes_as_primitive(byte_slice, *endian, |num: &mut u16| {
                    ui.add(egui::DragValue::new(num));
                });
            }
            StructPrimitive::I32 => {
                with_bytes_as_primitive(byte_slice, *endian, |num: &mut i32| {
                    ui.add(egui::DragValue::new(num));
                });
            }
            StructPrimitive::U32 => {
                with_bytes_as_primitive(byte_slice, *endian, |num: &mut u32| {
                    ui.add(egui::DragValue::new(num));
                });
            }
            StructPrimitive::I64 => {
                with_bytes_as_primitive(byte_slice, *endian, |num: &mut i64| {
                    ui.add(egui::DragValue::new(num));
                });
            }
            StructPrimitive::U64 => {
                with_bytes_as_primitive(byte_slice, *endian, |num: &mut u64| {
                    ui.add(egui::DragValue::new(num));
                });
            }
            StructPrimitive::F32 => {
                with_bytes_as_primitive(byte_slice, *endian, |num: &mut f32| {
                    ui.add(egui::DragValue::new(num));
                });
            }
            StructPrimitive::F64 => {
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
