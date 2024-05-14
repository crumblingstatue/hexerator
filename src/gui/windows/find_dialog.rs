use {
    super::{WinCtx, WindowOpen},
    crate::{
        app::{get_clipboard_string, set_clipboard_string},
        damage_region::DamageRegion,
        gui::{
            message_dialog::{Icon, MessageDialog},
            windows::region_context_menu,
        },
        meta::{
            find_most_specific_region_for_offset,
            value_type::{
                EndianedPrimitive, F32Be, F32Le, F64Be, F64Le, I16Be, I16Le, I32Be, I32Le, I64Be,
                I64Le, U16Be, U16Le, U32Be, U32Le, U64Be, U64Le, ValueType, I8, U8,
            },
            Bookmark, Meta,
        },
        parse_radix::parse_guess_radix,
        shell::{msg_fail, msg_if_fail},
    },
    egui::{Align, Ui},
    egui_extras::{Column, Size, StripBuilder, TableBuilder},
    itertools::Itertools,
    std::{collections::HashMap, error::Error, str::FromStr},
    strum::{EnumIter, IntoEnumIterator, IntoStaticStr},
};

#[derive(Default, Debug, PartialEq, Eq, EnumIter, IntoStaticStr)]
pub enum FindType {
    I8,
    #[default]
    U8,
    I16Le,
    I16Be,
    U16Le,
    U16Be,
    I32Le,
    I32Be,
    U32Le,
    U32Be,
    I64Le,
    I64Be,
    U64Le,
    U64Be,
    F32Le,
    F32Be,
    F64Le,
    F64Be,
    Ascii,
    StringDiff,
    /// Equivalence pattern
    EqPattern,
    HexString,
}

impl FindType {
    fn to_value_type(&self) -> ValueType {
        match self {
            FindType::I8 => ValueType::I8(I8),
            FindType::U8 => ValueType::U8(U8),
            FindType::I16Le => ValueType::I16Le(I16Le),
            FindType::I16Be => ValueType::I16Be(I16Be),
            FindType::U16Le => ValueType::U16Le(U16Le),
            FindType::U16Be => ValueType::U16Be(U16Be),
            FindType::I32Le => ValueType::I32Le(I32Le),
            FindType::I32Be => ValueType::I32Be(I32Be),
            FindType::U32Le => ValueType::U32Le(U32Le),
            FindType::U32Be => ValueType::U32Be(U32Be),
            FindType::I64Le => ValueType::I64Le(I64Le),
            FindType::I64Be => ValueType::I64Be(I64Be),
            FindType::U64Le => ValueType::U64Le(U64Le),
            FindType::U64Be => ValueType::U64Be(U64Be),
            FindType::F32Le => ValueType::F32Le(F32Le),
            FindType::F32Be => ValueType::F32Be(F32Be),
            FindType::F64Le => ValueType::F64Le(F64Le),
            FindType::F64Be => ValueType::F64Be(F64Be),
            FindType::Ascii => ValueType::None,
            FindType::StringDiff => ValueType::None,
            FindType::EqPattern => ValueType::None,
            FindType::HexString => ValueType::U8(U8),
        }
    }
}

#[derive(Default)]
pub struct FindDialog {
    pub open: WindowOpen,
    pub find_input: String,
    pub replace_input: String,
    /// Results, as a Vec that can be indexed. Needed because of search cursor.
    pub results_vec: Vec<usize>,
    /// Used to keep track of previous/next result to go to
    pub result_cursor: usize,
    /// When Some, the results list should be scrolled to the offset of that result
    pub scroll_to: Option<usize>,
    pub filter_results: bool,
    pub find_type: FindType,
    /// Used for increased/decreased unknown value search
    pub data_snapshot: Vec<u8>,
    /// Reload the source before search
    pub reload_before_search: bool,
    /// Only search in selection
    pub selection_only: bool,
}

impl super::Window for FindDialog {
    fn ui(&mut self, WinCtx { ui, gui, app, .. }: WinCtx) {
        ui.horizontal(|ui| {
            egui::ComboBox::new("type_combo", "Data type")
                .selected_text(<&str>::from(&self.find_type))
                .show_ui(ui, |ui| {
                    for type_ in FindType::iter() {
                        let label = <&str>::from(&type_);
                        ui.selectable_value(&mut self.find_type, type_, label);
                    }
                });
            ui.checkbox(&mut self.reload_before_search, "Reload")
                .on_hover_text("Reload source before every search");
            ui.checkbox(&mut self.selection_only, "Selection only")
                .on_hover_text("Only search in selection");
        });
        let re = ui.add(egui::TextEdit::singleline(&mut self.find_input).hint_text("🔍 Find"));
        if self.open.just_now() {
            re.request_focus();
        }
        if re.lost_focus() && ui.input(|inp| inp.key_pressed(egui::Key::Enter)) {
            if self.reload_before_search {
                msg_if_fail(app.reload(), "Failed to reload", &mut gui.msg_dialog);
            }
            let (data, offs) = if self.selection_only
                && let Some(sel) = app.hex_ui.selection()
            {
                (&app.data[sel.begin..=sel.end], sel.begin)
            } else {
                (&app.data[..], 0)
            };
            msg_if_fail(
                do_search(data, offs, self, gui),
                "Search failed",
                &mut gui.msg_dialog,
            );
            if let Some(&off) = self.results_vec.first() {
                app.search_focus(off);
            }
        }
        if self.find_type == FindType::Ascii {
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut self.replace_input).hint_text("🔁 Replace"));
                if ui
                    .add_enabled(
                        !self.results_vec.is_empty(),
                        egui::Button::new("Replace all"),
                    )
                    .clicked()
                {
                    let replace_data = self.replace_input.as_bytes();
                    for &offset in &self.results_vec {
                        app.data[offset..offset + replace_data.len()].copy_from_slice(replace_data);
                    }
                }
            });
        }
        ui.checkbox(&mut self.filter_results, "Filter results")
            .on_hover_text("Base search on existing results");
        StripBuilder::new(ui)
            .size(Size::initial(400.0))
            .size(Size::exact(20.0))
            .size(Size::exact(20.0))
            .vertical(|mut strip| {
                strip.cell(|ui| {
                    ui.style_mut().wrap = Some(false);
                    let mut action = Action::None;
                    TableBuilder::new(ui)
                        .striped(true)
                        .columns(Column::auto(), 3)
                        .column(Column::remainder())
                        .resizable(true)
                        .header(16.0, |mut row| {
                            row.col(|ui| {
                                ui.label("Offset");
                            });
                            row.col(|ui| {
                                ui.label("Value");
                            });
                            row.col(|ui| {
                                ui.label("Region");
                            });
                            row.col(|ui| {
                                ui.label("Bookmark");
                            });
                        })
                        .body(|body| {
                            body.rows(20.0, self.results_vec.len(), |mut row| {
                                let i = row.index();
                                let off = self.results_vec[i];
                                let (_, col1_re) = row.col(|ui| {
                                    let re = ui
                                        .selectable_label(self.result_cursor == i, off.to_string());
                                    re.context_menu(|ui| {
                                        if ui.button("Remove from results").clicked() {
                                            action = Action::RemoveIdxFromResults(i);
                                            ui.close_menu();
                                        }
                                    });
                                    if re.clicked() {
                                        app.search_focus(off);
                                        self.result_cursor = i;
                                    }
                                });
                                row.col(|ui| {
                                    let damage = match self.find_type {
                                        FindType::I8 => {
                                            data_value_label::<I8>(ui, &mut app.data, off)
                                        }
                                        FindType::U8 => {
                                            data_value_label::<U8>(ui, &mut app.data, off)
                                        }
                                        FindType::I16Le => {
                                            data_value_label::<I16Le>(ui, &mut app.data, off)
                                        }
                                        FindType::I16Be => {
                                            data_value_label::<I16Be>(ui, &mut app.data, off)
                                        }
                                        FindType::U16Le => {
                                            data_value_label::<U16Le>(ui, &mut app.data, off)
                                        }
                                        FindType::U16Be => {
                                            data_value_label::<U16Be>(ui, &mut app.data, off)
                                        }
                                        FindType::I32Le => {
                                            data_value_label::<I32Le>(ui, &mut app.data, off)
                                        }
                                        FindType::I32Be => {
                                            data_value_label::<I32Be>(ui, &mut app.data, off)
                                        }
                                        FindType::U32Le => {
                                            data_value_label::<U32Le>(ui, &mut app.data, off)
                                        }
                                        FindType::U32Be => {
                                            data_value_label::<U32Be>(ui, &mut app.data, off)
                                        }
                                        FindType::I64Le => {
                                            data_value_label::<I64Le>(ui, &mut app.data, off)
                                        }
                                        FindType::I64Be => {
                                            data_value_label::<I64Be>(ui, &mut app.data, off)
                                        }
                                        FindType::U64Le => {
                                            data_value_label::<U64Le>(ui, &mut app.data, off)
                                        }
                                        FindType::U64Be => {
                                            data_value_label::<U64Be>(ui, &mut app.data, off)
                                        }
                                        FindType::F32Le => {
                                            data_value_label::<F32Le>(ui, &mut app.data, off)
                                        }
                                        FindType::F32Be => {
                                            data_value_label::<F32Be>(ui, &mut app.data, off)
                                        }
                                        FindType::F64Le => {
                                            data_value_label::<F64Le>(ui, &mut app.data, off)
                                        }
                                        FindType::F64Be => {
                                            data_value_label::<F64Be>(ui, &mut app.data, off)
                                        }
                                        FindType::Ascii => {
                                            data_value_label::<U8>(ui, &mut app.data, off)
                                        }
                                        FindType::HexString => {
                                            data_value_label::<U8>(ui, &mut app.data, off)
                                        }
                                        FindType::StringDiff => {
                                            data_value_label::<U8>(ui, &mut app.data, off)
                                        }
                                        FindType::EqPattern => {
                                            data_value_label::<U8>(ui, &mut app.data, off)
                                        }
                                    };
                                    if let Some(damage) = damage {
                                        app.edit_state.widen_dirty_region(damage);
                                    }
                                });
                                row.col(|ui| {
                                    match find_most_specific_region_for_offset(
                                        &app.meta_state.meta.low.regions,
                                        off,
                                    ) {
                                        Some(key) => {
                                            let reg = &app.meta_state.meta.low.regions[key];
                                            let ctx_menu = |ui: &mut egui::Ui| {
                                                region_context_menu(
                                                    ui,
                                                    reg,
                                                    key,
                                                    &app.meta_state.meta,
                                                    &mut app.cmd,
                                                    &mut gui.cmd,
                                                );
                                                ui.separator();
                                                if ui.button("Remove region from results").clicked()
                                                {
                                                    action = Action::RemoveRegionFromResults(key);
                                                    ui.close_menu();
                                                }
                                            };
                                            let re = ui.link(&reg.name);
                                            re.context_menu(ctx_menu);
                                            if re.clicked() {
                                                gui.win.regions.open.set(true);
                                                gui.win.regions.selected_key = Some(key);
                                            }
                                        }
                                        None => {
                                            ui.label("[no region]");
                                        }
                                    }
                                });
                                row.col(|ui| {
                                    match Meta::bookmark_for_offset(
                                        &app.meta_state.meta.bookmarks,
                                        off,
                                    ) {
                                        Some((bm_idx, bm)) => {
                                            if ui.link(&bm.label).on_hover_text(&bm.desc).clicked()
                                            {
                                                gui.win.bookmarks.open.set(true);
                                                gui.win.bookmarks.selected = Some(bm_idx);
                                            }
                                        }
                                        None => {
                                            if ui
                                                .button("✚")
                                                .on_hover_text("Add new bookmark")
                                                .clicked()
                                            {
                                                let idx = app.meta_state.meta.bookmarks.len();
                                                app.meta_state.meta.bookmarks.push(Bookmark {
                                                    offset: off,
                                                    label: "New bookmark".into(),
                                                    desc: String::new(),
                                                    value_type: gui
                                                        .win
                                                        .find
                                                        .find_type
                                                        .to_value_type(),
                                                });
                                                gui.win.bookmarks.open.set(true);
                                                gui.win.bookmarks.selected = Some(idx);
                                                gui.win.bookmarks.edit_name = true;
                                                gui.win.bookmarks.focus_text_edit = true;
                                            }
                                        }
                                    }
                                });
                                if let Some(scroll_off) = self.scroll_to
                                    && scroll_off == i
                                {
                                    // We use center align, because it keeps the selected element in
                                    // view at all times, preventing the issue of it becoming out
                                    // of view, and scroll_to_me not being called because of that.
                                    col1_re.scroll_to_me(Some(Align::Center));
                                    self.scroll_to = None;
                                }
                            });
                        });
                    match action {
                        Action::None => {}
                        Action::RemoveRegionFromResults(key) => {
                            let reg = &app.meta_state.meta.low.regions[key];
                            gui.win
                                .find
                                .results_vec
                                .retain(|&idx| !reg.region.contains(idx));
                        }
                        Action::RemoveIdxFromResults(idx) => {
                            self.results_vec.remove(idx);
                        }
                    }
                });
                strip.cell(|ui| {
                    ui.horizontal(|ui| {
                        ui.set_enabled(!self.results_vec.is_empty());
                        if (ui.button("Previous (P)").clicked()
                            || ui.input(|inp| inp.key_pressed(egui::Key::P)))
                            && self.result_cursor > 0
                            && !self.results_vec.is_empty()
                        {
                            self.result_cursor -= 1;
                            let off = self.results_vec[self.result_cursor];
                            app.search_focus(off);
                            self.scroll_to = Some(self.result_cursor);
                        }
                        ui.label((self.result_cursor + 1).to_string());
                        if (ui.button("Next (N)").clicked()
                            || ui.input(|inp| inp.key_pressed(egui::Key::N)))
                            && self.result_cursor + 1 < self.results_vec.len()
                        {
                            self.result_cursor += 1;
                            let off = self.results_vec[self.result_cursor];
                            app.search_focus(off);
                            self.scroll_to = Some(self.result_cursor);
                        }
                        ui.label(format!("{} results", self.results_vec.len()));
                    });
                });
                strip.cell(|ui| {
                    ui.horizontal(|ui| {
                        if ui.button("Copy offsets").clicked() {
                            let s = gui
                                .win
                                .find
                                .results_vec
                                .iter()
                                .map(ToString::to_string)
                                .join(" ");
                            set_clipboard_string(&mut app.clipboard, &mut gui.msg_dialog, &s);
                        }
                        if ui.button("Paste offsets").clicked() {
                            let s = get_clipboard_string(&mut app.clipboard, &mut gui.msg_dialog);
                            let offsets: Result<Vec<usize>, _> =
                                s.split_ascii_whitespace().map(|s| s.parse()).collect();
                            match offsets {
                                Ok(offs) => self.results_vec = offs,
                                Err(e) => {
                                    msg_fail(&e, "failed to parse offsets", &mut gui.msg_dialog)
                                }
                            }
                        }
                        if ui.button("🗑 Clear").clicked() {
                            self.results_vec.clear();
                        }
                        // We don't want to highlight results by default, because
                        // it (at the very least) doubles memory usage for find results,
                        // which can be catastrophic for really large searches.
                        if ui.button("💡 Highlight").clicked() {
                            gui.highlight_set.clear();
                            for &offset in &self.results_vec {
                                gui.highlight_set.insert(offset);
                            }
                        }
                    });
                });
            });
    }

    fn title(&self) -> &str {
        "Find"
    }
}

trait SliceExt<T> {
    #[expect(dead_code, reason = "Could be useful in the future")]
    fn get_array<const N: usize>(&self, offset: usize) -> Option<&[T; N]>;
    fn get_array_mut<const N: usize>(&mut self, offset: usize) -> Option<&mut [T; N]>;
}

impl<T> SliceExt<T> for [T] {
    fn get_array<const N: usize>(&self, offset: usize) -> Option<&[T; N]> {
        self.get(offset..offset + N)?.try_into().ok()
    }
    fn get_array_mut<const N: usize>(&mut self, offset: usize) -> Option<&mut [T; N]> {
        self.get_mut(offset..offset + N)?.try_into().ok()
    }
}

fn data_value_label<N: EndianedPrimitive>(
    ui: &mut Ui,
    data: &mut [u8],
    off: usize,
) -> Option<DamageRegion>
where
    [(); N::BYTE_LEN]:,
{
    let Some(data) = data.get_array_mut(off) else {
        ui.label("!!").on_hover_text("Out of bounds");
        return None;
    };
    let mut n = N::from_bytes(*data);
    if ui.add(egui::DragValue::new(&mut n)).changed() {
        *data = N::to_bytes(n);
        return Some(DamageRegion::Range(off..off + N::BYTE_LEN));
    }
    None
}

enum Action {
    None,
    RemoveRegionFromResults(crate::meta::RegionKey),
    RemoveIdxFromResults(usize),
}

fn do_search(
    data: &[u8],
    initial_offset: usize,
    win: &mut FindDialog,
    gui: &mut crate::gui::Gui,
) -> anyhow::Result<()> {
    // Reset the result cursor, so it's not out of bounds if new results_vec is smaller
    // TODO: Review everything to use `initial_offset` correctly
    win.result_cursor = 0;
    if !win.filter_results {
        win.results_vec.clear();
    }
    match win.find_type {
        FindType::I8 => find_num::<I8>(win, data)?,
        FindType::U8 => find_u8(win, data, initial_offset, &mut gui.msg_dialog),
        FindType::I16Le => find_num::<I16Le>(win, data)?,
        FindType::I16Be => find_num::<I16Be>(win, data)?,
        FindType::U16Le => find_num::<U16Le>(win, data)?,
        FindType::U16Be => find_num::<U16Be>(win, data)?,
        FindType::I32Le => find_num::<I32Le>(win, data)?,
        FindType::I32Be => find_num::<I32Be>(win, data)?,
        FindType::U32Le => find_num::<U32Le>(win, data)?,
        FindType::U32Be => find_num::<U32Be>(win, data)?,
        FindType::I64Le => find_num::<I64Le>(win, data)?,
        FindType::I64Be => find_num::<I64Be>(win, data)?,
        FindType::U64Le => find_num::<U64Le>(win, data)?,
        FindType::U64Be => find_num::<U64Be>(win, data)?,
        FindType::F32Le => find_num::<F32Le>(win, data)?,
        FindType::F32Be => find_num::<F32Be>(win, data)?,
        FindType::F64Le => find_num::<F64Le>(win, data)?,
        FindType::F64Be => find_num::<F64Be>(win, data)?,
        FindType::Ascii => {
            for offset in memchr::memmem::find_iter(data, &win.find_input) {
                win.results_vec.push(initial_offset + offset);
            }
        }
        FindType::HexString => {
            let fun = |offset| {
                win.results_vec.push(initial_offset + offset);
            };
            let result = crate::find_util::find_hex_string(&win.find_input, data, fun);
            msg_if_fail(result, "Hex string search error", &mut gui.msg_dialog);
        }
        FindType::StringDiff => {
            let diff = ascii_to_diff_pattern(win.find_input.as_bytes());
            let mut off = 0;
            while let Some(offset) = find_diff_pattern(&data[off..], &diff) {
                off += offset;
                win.results_vec.push(initial_offset + off);
                off += diff.len();
            }
        }
        FindType::EqPattern => {
            let needle = make_eq_pattern_needle(&win.find_input);
            let mut off = 0;
            while let Some(offset) = find_eq_pattern_needle(&needle, &data[off..]) {
                off += offset;
                win.results_vec.push(initial_offset + off);
                off += needle.len();
            }
        }
    }
    Ok(())
}

fn make_eq_pattern_needle(pattern: &str) -> Vec<u8> {
    let mut needle = Vec::new();
    let mut map = HashMap::new();
    let mut uniq_counter = 0u8;
    for b in pattern.bytes() {
        let val = map.entry(b).or_insert_with(|| {
            let val = uniq_counter;
            uniq_counter += 1;
            val
        });
        needle.push(*val);
    }
    needle
}

#[test]
fn test_make_eq_pattern_needle() {
    assert_eq!(
        make_eq_pattern_needle("ABCDBEFFG"),
        &[0, 1, 2, 3, 1, 4, 5, 5, 6]
    );
    assert_eq!(
        make_eq_pattern_needle("abcdefggheijkbbl"),
        &[0, 1, 2, 3, 4, 5, 6, 6, 7, 4, 8, 9, 10, 1, 1, 11]
    );
}

#[cfg(test)]
fn find_eq_pattern(pattern: &str, data: &[u8]) -> Option<usize> {
    let needle = make_eq_pattern_needle(pattern);
    find_eq_pattern_needle(&needle, data)
}

fn find_eq_pattern_needle(needle: &[u8], data: &[u8]) -> Option<usize> {
    for window in data.windows(needle.len()) {
        if eq_pattern_needle_matches(needle, window) {
            return Some(window.as_ptr() as usize - data.as_ptr() as usize);
        }
    }
    None
}

fn eq_pattern_needle_matches(needle: &[u8], data: &[u8]) -> bool {
    for (n1, d1) in needle.iter().zip(data.iter()) {
        for (n2, d2) in needle.iter().zip(data.iter()) {
            if (n1 == n2) != (d1 == d2) {
                return false;
            }
        }
    }
    true
}

#[test]
fn test_find_eq_pattern() {
    assert_eq!(find_eq_pattern("ABCDBEFFG", b"I AM GOOD"), Some(0));
    assert_eq!(
        find_eq_pattern("abcdefggheijkbbk", b"Hello world, very cool indeed"),
        Some(13)
    );
}

#[allow(clippy::cast_possible_wrap)]
fn ascii_to_diff_pattern(ascii: &[u8]) -> Vec<i8> {
    ascii
        .array_windows()
        .map(|[a, b]| *b as i8 - *a as i8)
        .collect()
}

#[allow(clippy::cast_possible_wrap)]
fn find_diff_pattern(haystack: &[u8], pat: &[i8]) -> Option<usize> {
    assert!(pat.len() <= haystack.len());
    let mut pat_cur = 0;
    for (i, [a, b]) in haystack.array_windows().enumerate() {
        let Some(diff) = (*b as i8).checked_sub(*a as i8) else {
            pat_cur = 0;
            continue;
        };
        if diff == pat[pat_cur] {
            pat_cur += 1;
        } else {
            pat_cur = 0;
        }
        if pat_cur == pat.len() {
            return Some((i + 1) - pat.len());
        }
    }
    None
}

#[test]
fn test_ascii_to_diff_pattern() {
    assert_eq!(
        ascii_to_diff_pattern(b"jonathan"),
        vec![5, -1, -13, 19, -12, -7, 13]
    );
}

#[test]
#[allow(clippy::unwrap_used)]
fn test_find_diff_pattern() {
    let key = "jonathan";
    let pat = ascii_to_diff_pattern(key.as_bytes());
    let s = "I handed the key to jonathan. He didn't like the way I said jonathan to him.";
    let mut off = 0;
    off += find_diff_pattern(&s.as_bytes()[off..], &pat).unwrap();
    assert_eq!(&s[off..off + key.len()], key);
    off += pat.len();
    off += find_diff_pattern(&s.as_bytes()[off..], &pat).unwrap();
    assert_eq!(&s[off..off + key.len()], key);
}

fn find_num<N: EndianedPrimitive>(win: &mut FindDialog, data: &[u8]) -> Result<(), anyhow::Error>
where
    [(); N::BYTE_LEN]:,
    <<N as EndianedPrimitive>::Primitive as FromStr>::Err: Error + Send + Sync,
{
    find_num_raw::<N>(&win.find_input, data, |offset| {
        win.results_vec.push(offset);
    })
}

pub(crate) fn find_num_raw<N: EndianedPrimitive>(
    input: &str,
    data: &[u8],
    mut f: impl FnMut(usize),
) -> anyhow::Result<()>
where
    [(); N::BYTE_LEN]:,
    <<N as EndianedPrimitive>::Primitive as FromStr>::Err: Error + Send + Sync,
{
    let n: N::Primitive = input.parse()?;
    let bytes = N::to_bytes(n);
    for offset in memchr::memmem::find_iter(data, &bytes) {
        f(offset)
    }
    Ok(())
}

fn find_u8(dia: &mut FindDialog, data: &[u8], initial_offset: usize, msg: &mut MessageDialog) {
    // TODO: This is probably a minefield for initial_offset shenanigans.
    // Need to review carefully
    match dia.find_input.as_str() {
        "?" => {
            dia.data_snapshot = data.to_vec();
            dia.results_vec.clear();
            for i in 0..data.len() {
                dia.results_vec.push(initial_offset + i);
            }
        }
        ">" => {
            if dia.filter_results {
                dia.results_vec.retain(|&offset| {
                    data[offset - initial_offset] > dia.data_snapshot[offset - initial_offset]
                });
            } else {
                for (i, (&new, &old)) in data.iter().zip(dia.data_snapshot.iter()).enumerate() {
                    if new > old {
                        dia.results_vec.push(i);
                    }
                }
            }
            dia.data_snapshot = data.to_vec();
        }
        "=" => {
            if dia.filter_results {
                dia.results_vec.retain(|&offset| {
                    data[offset - initial_offset] == dia.data_snapshot[offset - initial_offset]
                });
            } else {
                for (i, (&new, &old)) in data.iter().zip(dia.data_snapshot.iter()).enumerate() {
                    if new == old {
                        dia.results_vec.push(i);
                    }
                }
            }
            dia.data_snapshot = data.to_vec();
        }
        "!=" => {
            if dia.filter_results {
                dia.results_vec.retain(|&offset| {
                    data[offset - initial_offset] != dia.data_snapshot[offset - initial_offset]
                });
            } else {
                for (i, (&new, &old)) in data.iter().zip(dia.data_snapshot.iter()).enumerate() {
                    if new == old {
                        dia.results_vec.push(i);
                    }
                }
            }
            dia.data_snapshot = data.to_vec();
        }
        "<" => {
            if dia.filter_results {
                dia.results_vec.retain(|&offset| {
                    data[offset - initial_offset] < dia.data_snapshot[offset - initial_offset]
                });
            } else {
                for (i, (&new, &old)) in data.iter().zip(dia.data_snapshot.iter()).enumerate() {
                    if new < old {
                        dia.results_vec.push(i);
                    }
                }
            }
            dia.data_snapshot = data.to_vec();
        }
        _ => match parse_guess_radix(&dia.find_input) {
            Ok(needle) => {
                if dia.filter_results {
                    let results_vec_clone = dia.results_vec.clone();
                    dia.results_vec.clear();
                    u8_search(
                        dia,
                        results_vec_clone.iter().map(|&off| (off, data[off])),
                        initial_offset,
                        needle,
                    );
                } else {
                    u8_search(
                        dia,
                        data.iter().cloned().enumerate(),
                        initial_offset,
                        needle,
                    );
                }
            }
            Err(e) => msg.open(Icon::Error, "Parse error", e.to_string()),
        },
    }
}

fn u8_search(
    dialog: &mut FindDialog,
    haystack: impl Iterator<Item = (usize, u8)>,
    initial_offset: usize,
    needle: u8,
) {
    for (offset, byte) in haystack {
        if byte == needle {
            dialog.results_vec.push(initial_offset + offset);
        }
    }
}
