use {
    super::{
        message_dialog::MessageDialog, regions_window::region_context_menu,
        window_open::WindowOpen, Gui,
    },
    crate::{
        app::{edit_state::EditState, set_clipboard_string, App},
        damage_region::DamageRegion,
        meta::{
            find_most_specific_region_for_offset,
            value_type::{
                EndianedPrimitive, F32Be, F32Le, F64Be, F64Le, I16Be, I16Le, I32Be, I32Le, I64Be,
                I64Le, StringMap, U16Be, U16Le, U32Be, U32Le, U64Be, U64Le, ValueType, I8, U8,
            },
            Bookmark,
        },
        shell::{msg_fail, msg_if_fail},
    },
    anyhow::Context,
    egui::{text::CursorRange, ScrollArea, Ui},
    egui_extras::{Column, TableBuilder},
    gamedebug_core::per,
    num_traits::AsPrimitive,
    std::mem::discriminant,
};

#[derive(Default)]
pub struct BookmarksWindow {
    pub open: WindowOpen,
    pub selected: Option<usize>,
    pub edit_name: bool,
    pub focus_text_edit: bool,
    value_type_string_buf: String,
    name_filter_string: String,
}

impl BookmarksWindow {
    pub fn ui(ui: &mut Ui, gui: &mut Gui, app: &mut App) {
        let win = &mut gui.bookmarks_window;
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut win.name_filter_string).hint_text("Filter by name"),
            );
            if ui.button("Highlight all").clicked() {
                gui.highlight_set.clear();
                for bm in &app.meta_state.meta.bookmarks {
                    gui.highlight_set.insert(bm.offset);
                }
            }
        });
        let mut action = Action::None;
        ScrollArea::vertical().max_height(500.0).show(ui, |ui| {
            TableBuilder::new(ui)
                .columns(Column::auto(), 4)
                .column(Column::remainder())
                .striped(true)
                .resizable(true)
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
                                .to_ascii_lowercase()
                                .contains(&win.name_filter_string.to_ascii_lowercase())
                    });
                    body.rows(20.0, keys.len(), |mut row| {
                        let idx = keys[row.index()];
                        row.col(|ui| {
                            let re = ui.selectable_label(
                                win.selected == Some(idx),
                                &app.meta_state.meta.bookmarks[idx].label,
                            );
                            re.context_menu(|ui| {
                                if ui.button("Copy name to clipboard").clicked() {
                                    set_clipboard_string(
                                        &mut app.clipboard,
                                        &mut gui.msg_dialog,
                                        &app.meta_state.meta.bookmarks[idx].label,
                                    );
                                    ui.close_menu();
                                }
                            });
                            if re.clicked() {
                                win.selected = Some(idx);
                            }
                        });
                        row.col(|ui| {
                            let offset = app.meta_state.meta.bookmarks[idx].offset;
                            let ctx_menu = |ui: &mut egui::Ui| {
                                if ui.button("Copy to clipboard").clicked() {
                                    set_clipboard_string(
                                        &mut app.clipboard,
                                        &mut gui.msg_dialog,
                                        &offset.to_string(),
                                    );
                                    ui.close_menu();
                                }
                            };
                            let re = ui.link(offset.to_string());
                            re.context_menu(ctx_menu);
                            if re.clicked() {
                                action = Action::Goto(offset);
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
                                &mut app.clipboard,
                                &mut gui.msg_dialog,
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
                                let ctx_menu = |ui: &mut egui::Ui| {
                                    region_context_menu(
                                        ui,
                                        region,
                                        region_key,
                                        &app.meta_state.meta,
                                        &mut app.cmd,
                                        &mut gui.cmd,
                                    );
                                };
                                let re = ui.link(&region.name).on_hover_text(&region.desc);
                                re.context_menu(ctx_menu);
                                if re.clicked() {
                                    gui.regions_window.open.set(true);
                                    gui.regions_window.selected_key = Some(region_key);
                                }
                            } else {
                                ui.label("<no region>");
                            }
                        });
                    });
                });
        });
        if let Some(idx) = win.selected {
            let Some(mark) = app.meta_state.meta.bookmarks.get_mut(idx) else {
                per!("Invalid bookmark selection: {idx}");
                win.selected = None;
                return;
            };
            ui.separator();
            ui.horizontal(|ui| {
                if win.edit_name {
                    let mut out = egui::TextEdit::singleline(&mut mark.label).show(ui);
                    if out.response.lost_focus() {
                        win.edit_name = false;
                    }
                    if win.focus_text_edit {
                        out.response.request_focus();
                        out.state
                            .cursor
                            .set_range(Some(CursorRange::select_all(&out.galley)));
                        out.state.store(ui.ctx(), out.response.id);
                        win.focus_text_edit = false;
                    }
                } else {
                    ui.heading(&mark.label);
                }
                if ui.button("✏").clicked() {
                    win.edit_name ^= true;
                }
                if ui.button("⮩").on_hover_text("Jump").clicked() {
                    action = Action::Goto(mark.offset);
                }
            });
            ui.horizontal(|ui| {
                ui.label("Offset");
                ui.add(egui::DragValue::new(&mut mark.offset));
            });
            egui::ComboBox::new("type_combo", "value type")
                .selected_text(mark.value_type.label())
                .show_ui(ui, |ui| {
                    macro_rules! int_sel_vals {
                        ($($t:ident,)*) => {
                            $(
                                ui.selectable_value(
                                    &mut mark.value_type,
                                    ValueType::$t($t),
                                    ValueType::$t($t).label(),
                                );
                            )*
                        }
                    }
                    ui.selectable_value(
                        &mut mark.value_type,
                        ValueType::None,
                        ValueType::None.label(),
                    );
                    int_sel_vals! {
                        I8, U8,
                        I16Le, U16Le, I16Be, U16Be,
                        I32Le, U32Le, I32Be, U32Be,
                        I64Le, U64Le, I64Be, U64Be,
                        F32Le, F32Be, F64Le, F64Be,
                    }
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
            ui.horizontal(|ui| {
                ui.label("Value");
                let result = value_ui(
                    mark,
                    &mut app.data,
                    &mut app.edit_state,
                    ui,
                    &mut app.clipboard,
                    &mut gui.msg_dialog,
                );
                match result {
                    Ok(value_ui_action) => match (&value_ui_action, &action) {
                        (Action::None, Action::None) => {}
                        (Action::None, Action::Goto(_)) => {}
                        (Action::Goto(_), Action::None) => action = value_ui_action,
                        (Action::Goto(_), Action::Goto(_)) => {
                            msg_fail(
                                &"Conflicting goto action",
                                "Ui Action error",
                                &mut gui.msg_dialog,
                            );
                        }
                    },
                    Err(e) => msg_fail(&e, "Value ui error", &mut gui.msg_dialog),
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
                        && ui.input(|inp| inp.key_pressed(egui::Key::Enter));
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
            egui::ScrollArea::vertical()
                .id_source("desc_scroll")
                .max_height(200.0)
                .show(ui, |ui| {
                    ui.add(egui::TextEdit::multiline(&mut mark.desc).code_editor());
                });
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
            });
            win.selected = Some(app.meta_state.meta.bookmarks.len() - 1);
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
    cb: &mut arboard::Clipboard,
    msg: &mut MessageDialog,
) -> anyhow::Result<Action> {
    macro_rules! val_ui_dispatch {
        ($i:ident) => {
            $i.value_ui_for_self(bm, data, edit_state, ui, cb, msg)
                .to_action()
        };
    }
    Ok(match &bm.value_type {
        ValueType::None => Action::None,
        ValueType::I8(v) => val_ui_dispatch!(v),
        ValueType::U8(v) => val_ui_dispatch!(v),
        ValueType::I16Le(v) => val_ui_dispatch!(v),
        ValueType::U16Le(v) => val_ui_dispatch!(v),
        ValueType::I16Be(v) => val_ui_dispatch!(v),
        ValueType::U16Be(v) => val_ui_dispatch!(v),
        ValueType::I32Le(v) => val_ui_dispatch!(v),
        ValueType::U32Le(v) => val_ui_dispatch!(v),
        ValueType::I32Be(v) => val_ui_dispatch!(v),
        ValueType::U32Be(v) => val_ui_dispatch!(v),
        ValueType::I64Le(v) => val_ui_dispatch!(v),
        ValueType::U64Le(v) => val_ui_dispatch!(v),
        ValueType::I64Be(v) => val_ui_dispatch!(v),
        ValueType::U64Be(v) => val_ui_dispatch!(v),
        ValueType::F32Le(v) => val_ui_dispatch!(v),
        ValueType::F32Be(v) => val_ui_dispatch!(v),
        ValueType::F64Le(v) => val_ui_dispatch!(v),
        ValueType::F64Be(v) => val_ui_dispatch!(v),
        ValueType::StringMap(v) => val_ui_dispatch!(v),
    })
}

trait ValueTrait: EndianedPrimitive {
    /// Returns whether the value was changed.
    fn value_change_ui(
        &self,
        ui: &mut egui::Ui,
        bytes: &mut [u8; Self::BYTE_LEN],
        cb: &mut arboard::Clipboard,
        msg: &mut MessageDialog,
    ) -> ValueUiOutput<Self::Primitive>;
    fn value_ui_for_self(
        &self,
        bm: &Bookmark,
        data: &mut [u8],
        edit_state: &mut EditState,
        ui: &mut Ui,
        cb: &mut arboard::Clipboard,
        msg: &mut MessageDialog,
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
                let out = self.value_change_ui(ui, slice.try_into().unwrap(), cb, msg);
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

trait DefaultUi {}
impl DefaultUi for I8 {}
impl DefaultUi for U8 {}
impl DefaultUi for I16Le {}
impl DefaultUi for U16Le {}
impl DefaultUi for I16Be {}
impl DefaultUi for U16Be {}
impl DefaultUi for I32Le {}
impl DefaultUi for U32Le {}
impl DefaultUi for I32Be {}
impl DefaultUi for U32Be {}
impl DefaultUi for I64Le {}
impl DefaultUi for U64Le {}
impl DefaultUi for I64Be {}
impl DefaultUi for U64Be {}
impl DefaultUi for F32Le {}
impl DefaultUi for F32Be {}
impl DefaultUi for F64Le {}
impl DefaultUi for F64Be {}

impl<T: EndianedPrimitive + DefaultUi> ValueTrait for T {
    fn value_change_ui(
        &self,
        ui: &mut egui::Ui,
        bytes: &mut [u8; Self::BYTE_LEN],
        cb: &mut arboard::Clipboard,
        msg: &mut MessageDialog,
    ) -> ValueUiOutput<Self::Primitive> {
        let mut val = Self::from_bytes(*bytes);
        let mut action = UiAction::None;
        let act_mut = &mut action;
        let ctx_menu = move |ui: &mut egui::Ui| {
            if ui.button("Copy").clicked() {
                crate::app::set_clipboard_string(cb, msg, &val.to_string());
                ui.close_menu();
            }
            if ui.button("Jump").clicked() {
                ui.close_menu();
                *act_mut = UiAction::Goto(val);
            }
        };
        let re = ui.add(egui::DragValue::new(&mut val));
        re.context_menu(ctx_menu);
        let changed = if re.changed() {
            bytes.copy_from_slice(&Self::to_bytes(val));
            true
        } else {
            false
        };
        ValueUiOutput { changed, action }
    }
}

impl EndianedPrimitive for StringMap {
    type Primitive = u8;

    fn from_bytes(bytes: [u8; Self::BYTE_LEN]) -> Self::Primitive {
        bytes[0]
    }

    fn to_bytes(prim: Self::Primitive) -> [u8; Self::BYTE_LEN] {
        [prim]
    }

    fn label(&self) -> &'static str {
        "string map"
    }
}

impl ValueTrait for StringMap {
    fn value_change_ui(
        &self,
        ui: &mut egui::Ui,
        bytes: &mut [u8; Self::BYTE_LEN],
        _cb: &mut arboard::Clipboard,
        _msg: &mut MessageDialog,
    ) -> ValueUiOutput<Self::Primitive> {
        let val = &mut bytes[0];
        let mut s = String::new();
        let label = self.get(val).unwrap_or_else(|| {
            s = format!("[unmapped: {val}]");
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
impl<T: AsPrimitive<usize>> UiAction<T> {
    fn to_action(&self) -> Action {
        match self {
            UiAction::None => Action::None,
            &UiAction::Goto(val) => Action::Goto(val.as_()),
        }
    }
}
