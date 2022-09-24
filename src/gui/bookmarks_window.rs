use {
    super::{window_open::WindowOpen, Gui},
    crate::{
        app::{edit_state::EditState, App},
        damage_region::DamageRegion,
        meta::{find_most_specific_region_for_offset, value_type::ValueType, Bookmark},
        region_context_menu,
        shell::msg_if_fail,
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
            .columns(Size::remainder(), 4)
            .striped(true)
            .header(24.0, |mut row| {
                row.col(|ui| {
                    ui.label("Name");
                });
                row.col(|ui| {
                    ui.label("Offset");
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
                        let result = value_ui(
                            &app.meta_state.meta.bookmarks[idx],
                            &mut app.data,
                            &mut app.edit_state,
                            ui,
                        );
                        msg_if_fail(result, "Value ui failed", &mut gui.msg_dialog);
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
                    ui.selectable_value(&mut mark.value_type, ValueType::U8, ValueType::U8.label());
                    ui.selectable_value(
                        &mut mark.value_type,
                        ValueType::U16Le,
                        ValueType::U16Le.label(),
                    );
                    ui.selectable_value(
                        &mut mark.value_type,
                        ValueType::U64Le,
                        ValueType::U64Le.label(),
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
) -> anyhow::Result<()> {
    match &bm.value_type {
        ValueType::None => {}
        ValueType::U8 => match data.get_mut(bm.offset) {
            Some(byte) => {
                if ui.add(egui::DragValue::new(byte)).changed() {
                    edit_state.widen_dirty_region(DamageRegion::Single(bm.offset));
                }
            }
            None => {
                ui.label("??");
            }
        },
        ValueType::U16Le => match data.get_mut(bm.offset..bm.offset + 2) {
            Some(slice) => {
                let mut val = u16::from_le_bytes(slice.try_into()?);
                if ui.add(egui::DragValue::new(&mut val)).changed() {
                    slice.copy_from_slice(&val.to_le_bytes());
                    edit_state.widen_dirty_region(DamageRegion::Range(bm.offset..bm.offset + 2));
                }
            }
            None => {
                ui.label("??");
            }
        },
        ValueType::U64Le => match data.get_mut(bm.offset..bm.offset + 8) {
            Some(slice) => {
                let mut val = u64::from_le_bytes(slice.try_into()?);
                if ui.add(egui::DragValue::new(&mut val)).changed() {
                    slice.copy_from_slice(&val.to_le_bytes());
                    edit_state.widen_dirty_region(DamageRegion::Range(bm.offset..bm.offset + 8))
                }
            }
            None => {
                ui.label("??");
            }
        },
        ValueType::StringMap(list) => {
            let val = &mut data[bm.offset];
            let mut s = String::new();
            let label = list.get(val).unwrap_or_else(|| {
                s = format!("[unmapped: {}]", val);
                &s
            });
            egui::ComboBox::new("val_combo", "")
                .selected_text(label)
                .show_ui(ui, |ui| {
                    for (k, v) in list {
                        ui.selectable_value(val, *k, v);
                    }
                });
        }
    }
    Ok(())
}

enum Action {
    None,
    Goto(usize),
}
