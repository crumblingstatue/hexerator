use {
    super::{window_open::WindowOpen, Gui},
    crate::{
        app::{edit_state::EditState, App},
        damage_region::DamageRegion,
        meta::{
            find_most_specific_region_for_offset,
            value_type::{StringMap, U16Be, U16Le, U32Be, U32Le, U64Be, U64Le, ValueType, U8},
            Bookmark,
        },
        region_context_menu,
        shell::{msg_fail, msg_if_fail},
    },
    anyhow::Context,
    egui::{self, Ui},
    egui_extras::{Size, TableBuilder},
    std::mem::discriminant,
};

#[derive(Default)]
pub struct BookmarksWindow {
    pub open: WindowOpen,
    pub selected: Option<usize>,
    edit_name: bool,
    value_type_string_buf: String,
    name_filter_string: String,
}

impl BookmarksWindow {
    pub fn ui(ui: &mut Ui, gui: &mut Gui, app: &mut App) {
        let win = &mut gui.bookmarks_window;
        ui.add(egui::TextEdit::singleline(&mut win.name_filter_string).hint_text("Filter by name"));
        let mut action = Action::None;
        TableBuilder::new(ui)
            .columns(Size::remainder(), 5)
            .striped(true)
            .header(24.0, |mut row| {
                row.col(|ui| {
                    ui.label("Name");
                });
                row.col(|ui| {
                    ui.label("Offset");
                });
                row.col(|ui| {
                    ui.label("Type");
                });
                row.col(|ui| {
                    ui.label("Value");
                });
                row.col(|ui| {
                    ui.label("Region");
                });
            })
            .body(|body| {
                // Sort by offset
                let mut keys: Vec<usize> = (0..app.meta_state.meta.bookmarks.len()).collect();
                keys.sort_by_key(|&idx| app.meta_state.meta.bookmarks[idx].offset);
                keys.retain(|&k| {
                    win.name_filter_string.is_empty()
                        || app.meta_state.meta.bookmarks[k]
                            .label
                            .contains(&win.name_filter_string)
                });
                body.rows(20.0, keys.len(), |idx, mut row| {
                    let idx = keys[idx];
                    row.col(|ui| {
                        if ui
                            .selectable_label(
                                win.selected == Some(idx),
                                &app.meta_state.meta.bookmarks[idx].label,
                            )
                            .clicked()
                        {
                            win.selected = Some(idx);
                        }
                    });
                    row.col(|ui| {
                        if ui
                            .link(app.meta_state.meta.bookmarks[idx].offset.to_string())
                            .clicked()
                        {
                            action = Action::Goto(app.meta_state.meta.bookmarks[idx].offset);
                        }
                    });
                    row.col(|ui| {
                        ui.label(app.meta_state.meta.bookmarks[idx].value_type.label());
                    });
                    row.col(|ui| {
                        let result = value_ui(
                            &app.meta_state.meta.bookmarks[idx],
                            &mut app.data,
                            &mut app.edit_state,
                            ui,
                        );
                        match result {
                            Ok(action) => match action {
                                Action::None => {}
                                Action::Goto(offset) => app.search_focus(offset),
                            },
                            Err(e) => msg_fail(&e, "Value ui error", &mut gui.msg_dialog),
                        }
                    });
                    row.col(|ui| {
                        let off = app.meta_state.meta.bookmarks[idx].offset;
                        if let Some(region_key) = find_most_specific_region_for_offset(
                            &app.meta_state.meta.low.regions,
                            off,
                        ) {
                            let region = &app.meta_state.meta.low.regions[region_key];
                            let ctx_menu =
                                |ui: &mut egui::Ui| region_context_menu!(ui, app, region, action);
                            if ui
                                .link(&region.name)
                                .on_hover_text(&region.desc)
                                .context_menu(ctx_menu)
                                .clicked()
                            {
                                gui.regions_window.open.set(true);
                                gui.regions_window.selected_key = Some(region_key);
                            }
                        } else {
                            ui.label("<no region>");
                        }
                    });
                });
            });
        if let Some(idx) = win.selected {
            ui.separator();
            let mark = &mut app.meta_state.meta.bookmarks[idx];
            ui.horizontal(|ui| {
                if win.edit_name {
                    if ui.text_edit_singleline(&mut mark.label).lost_focus() {
                        win.edit_name = false;
                    }
                } else {
                    ui.heading(&mark.label);
                }
                if ui.button("✏").clicked() {
                    win.edit_name ^= true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Offset");
                ui.add(egui::DragValue::new(&mut mark.offset));
            });
            egui::ComboBox::new("type_combo", "value type")
                .selected_text(mark.value_type.label())
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut mark.value_type,
                        ValueType::None,
                        ValueType::None.label(),
                    );
                    ui.selectable_value(
                        &mut mark.value_type,
                        ValueType::U8(U8),
                        ValueType::U8(U8).label(),
                    );
                    ui.selectable_value(
                        &mut mark.value_type,
                        ValueType::U16Le(U16Le),
                        ValueType::U16Le(U16Le).label(),
                    );
                    ui.selectable_value(
                        &mut mark.value_type,
                        ValueType::U16Be(U16Be),
                        ValueType::U16Be(U16Be).label(),
                    );
                    ui.selectable_value(
                        &mut mark.value_type,
                        ValueType::U32Le(U32Le),
                        ValueType::U32Le(U32Le).label(),
                    );
                    ui.selectable_value(
                        &mut mark.value_type,
                        ValueType::U32Be(U32Be),
                        ValueType::U32Be(U32Be).label(),
                    );
                    ui.selectable_value(
                        &mut mark.value_type,
                        ValueType::U64Le(U64Le),
                        ValueType::U64Le(U64Le).label(),
                    );
                    ui.selectable_value(
                        &mut mark.value_type,
                        ValueType::U64Be(U64Be),
                        ValueType::U64Be(U64Be).label(),
                    );
                    let val = ValueType::StringMap(Default::default());
                    if ui
                        .selectable_label(
                            discriminant(&mark.value_type) == discriminant(&val),
                            val.label(),
                        )
                        .clicked()
                    {
                        mark.value_type = val;
                    }
                });
            #[expect(clippy::single_match, reason = "Want to add more variants in future")]
            match &mut mark.value_type {
                ValueType::StringMap(list) => {
                    let text_edit_finished = ui
                        .add(
                            egui::TextEdit::singleline(&mut win.value_type_string_buf)
                                .hint_text("key = value"),
                        )
                        .lost_focus()
                        && ui.input().key_pressed(egui::Key::Enter);
                    if text_edit_finished || ui.button("Set key = value").clicked() {
                        let result: anyhow::Result<()> = try {
                            let s = &win.value_type_string_buf;
                            let (k, v) = s.split_once('=').context("Missing `=`")?;
                            let k: u8 = k.trim().parse()?;
                            let v = v.trim().to_owned();
                            list.insert(k, v);
                        };
                        msg_if_fail(
                            result,
                            "Failed to set value list kvpair",
                            &mut gui.msg_dialog,
                        );
                    }
                }
                _ => {}
            }
            ui.heading("Description");
            ui.text_edit_multiline(&mut mark.desc);
            if ui.button("Delete").clicked() {
                app.meta_state.meta.bookmarks.remove(idx);
                win.selected = None;
            }
        }
        ui.separator();
        if ui.button("Add new at cursor").clicked() {
            app.meta_state.meta.bookmarks.push(Bookmark {
                offset: app.edit_state.cursor,
                label: format!("New bookmark at {}", app.edit_state.cursor),
                desc: String::new(),
                value_type: ValueType::None,
            })
        }
        match action {
            Action::None => {}
            Action::Goto(off) => {
                app.edit_state.cursor = off;
                app.center_view_on_offset(off);
                app.hex_ui.flash_cursor();
            }
        }
    }
}

fn value_ui(
    bm: &Bookmark,
    data: &mut [u8],
    edit_state: &mut EditState,
    ui: &mut Ui,
) -> anyhow::Result<Action> {
    macro val_ui_dispatch($i:ident) {
        $i.value_ui_for_self(bm, data, edit_state, ui)
            .try_into_action()?
    }
    Ok(match &bm.value_type {
        ValueType::None => Action::None,
        ValueType::U8(v) => val_ui_dispatch!(v),
        ValueType::U16Le(v) => val_ui_dispatch!(v),
        ValueType::U16Be(v) => val_ui_dispatch!(v),
        ValueType::U32Le(v) => val_ui_dispatch!(v),
        ValueType::U32Be(v) => val_ui_dispatch!(v),
        ValueType::U64Le(v) => val_ui_dispatch!(v),
        ValueType::U64Be(v) => val_ui_dispatch!(v),
        ValueType::StringMap(v) => val_ui_dispatch!(v),
    })
}

trait ValueTrait: EndianedPrimitive {
    /// Returns whether the value was changed.
    fn value_change_ui(
        &self,
        ui: &mut egui::Ui,
        bytes: &mut [u8; Self::BYTE_LEN],
    ) -> ValueUiOutput<Self::Primitive>;
    fn value_ui_for_self(
        &self,
        bm: &Bookmark,
        data: &mut [u8],
        edit_state: &mut EditState,
        ui: &mut Ui,
    ) -> UiAction<Self::Primitive>
    where
        [(); Self::BYTE_LEN]:,
    {
        let range = bm.offset..bm.offset + Self::BYTE_LEN;
        match data.get_mut(range.clone()) {
            Some(slice) => {
                #[expect(
                    clippy::unwrap_used,
                    reason = "If slicing is successful, we're guaranteed to have slice of right length"
                )]
                let out = self.value_change_ui(ui, slice.try_into().unwrap());
                if out.changed {
                    edit_state.widen_dirty_region(DamageRegion::Range(range));
                }
                out.action
            }
            None => {
                ui.label("??");
                UiAction::None
            }
        }
    }
}

struct ValueUiOutput<T> {
    changed: bool,
    action: UiAction<T>,
}

trait EndianedPrimitive {
    const BYTE_LEN: usize = std::mem::size_of::<Self::Primitive>();
    type Primitive: egui::emath::Numeric + std::fmt::Display + TryInto<usize>;
    fn from_bytes(bytes: [u8; Self::BYTE_LEN]) -> Self::Primitive;
    fn to_bytes(prim: Self::Primitive) -> [u8; Self::BYTE_LEN];
}

trait DefaultUi {}

impl EndianedPrimitive for U8 {
    type Primitive = u8;

    fn from_bytes(bytes: [u8; Self::BYTE_LEN]) -> Self::Primitive {
        bytes[0]
    }

    fn to_bytes(prim: Self::Primitive) -> [u8; Self::BYTE_LEN] {
        [prim]
    }
}

impl DefaultUi for U8 {}

impl EndianedPrimitive for U16Le {
    type Primitive = u16;

    fn from_bytes(bytes: [u8; Self::BYTE_LEN]) -> Self::Primitive {
        u16::from_le_bytes(bytes)
    }

    fn to_bytes(prim: Self::Primitive) -> [u8; Self::BYTE_LEN] {
        prim.to_le_bytes()
    }
}

impl EndianedPrimitive for U16Be {
    type Primitive = u16;

    fn from_bytes(bytes: [u8; Self::BYTE_LEN]) -> Self::Primitive {
        u16::from_be_bytes(bytes)
    }

    fn to_bytes(prim: Self::Primitive) -> [u8; Self::BYTE_LEN] {
        prim.to_be_bytes()
    }
}

impl DefaultUi for U16Le {}
impl DefaultUi for U16Be {}

impl EndianedPrimitive for U32Le {
    type Primitive = u32;

    fn from_bytes(bytes: [u8; Self::BYTE_LEN]) -> Self::Primitive {
        u32::from_le_bytes(bytes)
    }

    fn to_bytes(prim: Self::Primitive) -> [u8; Self::BYTE_LEN] {
        prim.to_le_bytes()
    }
}

impl EndianedPrimitive for U32Be {
    type Primitive = u32;

    fn from_bytes(bytes: [u8; Self::BYTE_LEN]) -> Self::Primitive {
        u32::from_be_bytes(bytes)
    }

    fn to_bytes(prim: Self::Primitive) -> [u8; Self::BYTE_LEN] {
        prim.to_be_bytes()
    }
}

impl DefaultUi for U32Le {}
impl DefaultUi for U32Be {}

impl EndianedPrimitive for U64Le {
    type Primitive = u64;

    fn from_bytes(bytes: [u8; Self::BYTE_LEN]) -> Self::Primitive {
        u64::from_le_bytes(bytes)
    }

    fn to_bytes(prim: Self::Primitive) -> [u8; Self::BYTE_LEN] {
        prim.to_le_bytes()
    }
}

impl EndianedPrimitive for U64Be {
    type Primitive = u64;

    fn from_bytes(bytes: [u8; Self::BYTE_LEN]) -> Self::Primitive {
        u64::from_be_bytes(bytes)
    }

    fn to_bytes(prim: Self::Primitive) -> [u8; Self::BYTE_LEN] {
        prim.to_be_bytes()
    }
}

impl DefaultUi for U64Le {}
impl DefaultUi for U64Be {}

impl EndianedPrimitive for StringMap {
    type Primitive = u8;

    fn from_bytes(bytes: [u8; Self::BYTE_LEN]) -> Self::Primitive {
        bytes[0]
    }

    fn to_bytes(prim: Self::Primitive) -> [u8; Self::BYTE_LEN] {
        [prim]
    }
}

impl<T: EndianedPrimitive + DefaultUi> ValueTrait for T {
    fn value_change_ui(
        &self,
        ui: &mut egui::Ui,
        bytes: &mut [u8; Self::BYTE_LEN],
    ) -> ValueUiOutput<Self::Primitive> {
        let mut val = Self::from_bytes(*bytes);
        let mut action = UiAction::None;
        let act_mut = &mut action;
        let ctx_menu = move |ui: &mut egui::Ui| {
            if ui.button("Copy").clicked() {
                ui.output().copied_text = val.to_string();
                ui.close_menu();
            }
            if ui.button("Jump").clicked() {
                ui.close_menu();
                *act_mut = UiAction::Goto(val);
            }
        };
        let changed = if ui
            .add(egui::DragValue::new(&mut val))
            .context_menu(ctx_menu)
            .changed()
        {
            bytes.copy_from_slice(&Self::to_bytes(val));
            true
        } else {
            false
        };
        ValueUiOutput { changed, action }
    }
}

impl ValueTrait for StringMap {
    fn value_change_ui(
        &self,
        ui: &mut egui::Ui,
        bytes: &mut [u8; Self::BYTE_LEN],
    ) -> ValueUiOutput<Self::Primitive> {
        let val = &mut bytes[0];
        let mut s = String::new();
        let label = self.get(val).unwrap_or_else(|| {
            s = format!("[unmapped: {}]", val);
            &s
        });
        let mut changed = false;
        egui::ComboBox::new("val_combo", "")
            .selected_text(label)
            .show_ui(ui, |ui| {
                for (k, v) in self {
                    if ui.selectable_value(val, *k, v).clicked() {
                        changed = true;
                    }
                }
            });
        ValueUiOutput {
            changed,
            action: UiAction::None,
        }
    }
}

enum Action {
    None,
    Goto(usize),
}

enum UiAction<T> {
    None,
    Goto(T),
}
impl<T: TryInto<usize> + Copy> UiAction<T> {
    fn try_into_action(&self) -> Result<Action, <T as TryInto<usize>>::Error> {
        Ok(match self {
            UiAction::None => Action::None,
            &UiAction::Goto(val) => Action::Goto(val.try_into()?),
        })
    }
}
