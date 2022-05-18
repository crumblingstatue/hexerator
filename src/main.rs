#![feature(let_chains, decl_macro)]

mod hex_conv;
mod input;
mod slice_ext;

use crate::slice_ext::SliceExt;
use egui_inspect::{derive::Inspect, inspect};
use egui_sfml::{
    egui::{
        self, color::rgb_from_hsv, Button, Layout, ScrollArea, TextEdit, TopBottomPanel, Window,
    },
    SfEgui,
};
use gamedebug_core::{imm_msg, per_msg, Info, PerEntry, IMMEDIATE, PERSISTENT};
use sfml::{
    graphics::{
        Color, Font, PrimitiveType, Rect, RectangleShape, RenderStates, RenderTarget, RenderWindow,
        Shape, Vertex,
    },
    system::Vector2,
    window::{mouse, ContextSettings, Event, Key, Style},
};

use crate::input::Input;

#[derive(PartialEq, Debug, Inspect)]
enum EditTarget {
    Hex,
    Text,
}

impl EditTarget {
    fn switch(&mut self) {
        *self = match self {
            EditTarget::Hex => EditTarget::Text,
            EditTarget::Text => EditTarget::Hex,
        }
    }
}

/// User interaction mode
///
/// There are 2 modes: View and Edit
#[derive(PartialEq, Debug, Inspect)]
enum InteractMode {
    /// Mode optimized for viewing the contents
    ///
    /// For example arrow keys scroll the content
    View,
    /// Mode optimized for editing the contents
    ///
    /// For example arrow keys move the cursor
    Edit,
}

#[derive(Default)]
struct FindDialog {
    open: bool,
    input: String,
    result_offsets: Vec<usize>,
    /// Used to keep track of previous/next result to go to
    result_cursor: usize,
    /// When Some, the results list should be scrolled to the offset of that result
    scroll_to: Option<usize>,
}

#[derive(Clone, Copy, Debug)]
struct Region {
    begin: usize,
    end: usize,
}

fn main() {
    let path = std::env::args_os()
        .nth(1)
        .expect("Need file path as argument");
    let mut data = std::fs::read(&path).unwrap();
    let mut w = RenderWindow::new(
        (1920, 1080),
        "Hexerator",
        Style::NONE,
        &ContextSettings::default(),
    );
    w.set_vertical_sync_enabled(true);
    w.set_position(Vector2::new(0, 0));
    let mut sf_egui = SfEgui::new(&w);
    let f = unsafe { Font::from_memory(include_bytes!("../DejaVuSansMono.ttf")).unwrap() };
    let mut vertices = Vec::new();
    let mut rows = 67;
    // Number of columns in the view
    let mut cols = 48;
    // Maximum number of visible hex columns that can be shown on screen.
    // ascii is double this amount.
    let mut max_visible_cols = 75;
    // The byte offset in the data from which the view starts viewing data from
    let mut starting_offset: usize = 0;
    // The top part where the top panel is. You should try to position stuff so it's not overdrawn
    // by the top panel
    let top_gap = 30;
    // The x pixel offset of the scrollable view
    let mut view_x: i64 = 0;
    // The y pixel offset of the scrollable view
    let mut view_y: i64 = -top_gap;
    // The amount scrolled per frame in view mode
    let mut scroll_speed = 4;
    let mut colorize = true;
    // The editing byte offset
    let mut cursor: usize = 0;
    // The value of the cursor on the previous frame. Used to determine when the cursor changes
    let mut cursor_prev_frame = cursor;
    let mut edit_target = EditTarget::Hex;
    let mut dirty = false;
    let mut row_height: u8 = 16;
    let mut col_width: u8 = 26;
    let mut show_text = true;
    let mut interact_mode = InteractMode::View;
    // The half digit when the user begins to type into a hex view
    let mut hex_edit_half_digit = None;
    let mut show_debug_panel = false;
    let mut u8_buf = String::new();
    let mut find_dialog = FindDialog::default();
    let mut selection: Option<Region> = None;
    let mut select_begin: Option<usize> = None;
    let mut fill_text = String::new();
    let backup_path = {
        let mut new = path.to_owned();
        new.push(".hexerator_bak");
        new
    };
    let mut input = Input::default();
    macro reload() {
        data = std::fs::read(&path).unwrap();
        dirty = false;
    }
    macro save() {
        std::fs::write(&path, &data).unwrap();
        dirty = false;
    }
    macro toggle_debug() {{
        show_debug_panel ^= true;
        gamedebug_core::toggle();
    }}
    macro ascii_display_x_offset() {
        cols as i64 * i64::from(col_width) + 12
    }

    while w.is_open() {
        // region: event handling
        while let Some(event) = w.poll_event() {
            input.update_from_event(&event);
            sf_egui.add_event(&event);
            let wants_pointer = sf_egui.context().wants_pointer_input();
            let wants_kb = sf_egui.context().wants_keyboard_input();
            if wants_kb {
                if event == Event::Closed {
                    w.close();
                }
                continue;
            }
            match event {
                Event::Closed => w.close(),
                Event::KeyPressed {
                    code, shift, ctrl, ..
                } => match code {
                    Key::Up => match interact_mode {
                        InteractMode::View => {
                            if ctrl {
                                starting_offset = starting_offset.saturating_sub(1);
                            }
                        }
                        InteractMode::Edit => {
                            cursor = cursor.saturating_sub(cols);
                        }
                    },
                    Key::Down => match interact_mode {
                        InteractMode::View => {
                            if ctrl {
                                starting_offset += 1;
                            }
                        }
                        InteractMode::Edit => {
                            if cursor + cols < data.len() {
                                cursor += cols;
                            }
                        }
                    },
                    Key::Left => {
                        if interact_mode == InteractMode::Edit {
                            cursor = cursor.saturating_sub(1)
                        } else if ctrl {
                            cols -= 1;
                        }
                    }
                    Key::Right => {
                        if interact_mode == InteractMode::Edit && cursor + 1 < data.len() {
                            cursor += 1;
                        } else if ctrl {
                            cols += 1;
                        }
                    }
                    Key::PageUp => match interact_mode {
                        InteractMode::View => {
                            view_y -= 1040;
                        }
                        InteractMode::Edit => {
                            let amount = rows * cols;
                            if starting_offset >= amount {
                                starting_offset -= amount;
                                if interact_mode == InteractMode::Edit {
                                    cursor = cursor.saturating_sub(amount);
                                }
                            } else {
                                starting_offset = 0
                            }
                        }
                    },
                    Key::PageDown => match interact_mode {
                        InteractMode::View => view_y += 1040,
                        InteractMode::Edit => {
                            let amount = rows * cols;
                            if starting_offset + amount < data.len() {
                                starting_offset += amount;
                                if interact_mode == InteractMode::Edit
                                    && cursor + amount < data.len()
                                {
                                    cursor += amount;
                                }
                            }
                        }
                    },
                    Key::Home => match interact_mode {
                        InteractMode::View => view_y = -top_gap,
                        InteractMode::Edit => {
                            starting_offset = 0;
                            cursor = 0;
                        }
                    },
                    Key::End => match interact_mode {
                        InteractMode::View => {
                            let data_pix_size = (data.len() / cols) as i64 * i64::from(row_height);
                            view_y = data_pix_size - 1040;
                        }
                        InteractMode::Edit => {
                            let pos = data.len() - rows * cols;
                            starting_offset = pos;
                            if interact_mode == InteractMode::Edit {
                                cursor = pos;
                            }
                        }
                    },
                    Key::Tab if shift => {
                        edit_target.switch();
                        hex_edit_half_digit = None;
                    }
                    Key::F1 => interact_mode = InteractMode::View,
                    Key::F2 => interact_mode = InteractMode::Edit,
                    Key::F12 => toggle_debug!(),
                    Key::Escape => {
                        hex_edit_half_digit = None;
                    }
                    Key::F if ctrl => {
                        find_dialog.open ^= true;
                    }
                    Key::S if ctrl => {
                        save!();
                    }
                    Key::R if ctrl => {
                        reload!();
                    }
                    _ => {}
                },
                Event::TextEntered { unicode } => match interact_mode {
                    InteractMode::Edit => match edit_target {
                        EditTarget::Hex => {
                            if unicode.is_ascii() {
                                let ascii = unicode as u8;
                                if (b'0'..=b'f').contains(&ascii) {
                                    match hex_edit_half_digit {
                                        Some(half) => {
                                            data[cursor] = hex_conv::merge_hex_halves(half, ascii);
                                            dirty = true;
                                            if cursor + 1 < data.len() {
                                                cursor += 1;
                                            }
                                            hex_edit_half_digit = None;
                                        }
                                        None => hex_edit_half_digit = Some(ascii),
                                    }
                                }
                            }
                        }
                        EditTarget::Text => {
                            if unicode.is_ascii() {
                                data[cursor] = unicode as u8;
                                dirty = true;
                                if cursor + 1 < data.len() {
                                    cursor += 1;
                                }
                            }
                        }
                    },
                    InteractMode::View => {}
                },
                Event::MouseButtonPressed { button, x, y } if !wants_pointer => {
                    if button == mouse::Button::Left {
                        let x: i64 = view_x + i64::from(x);
                        let y: i64 = view_y + i64::from(y);
                        per_msg!("x: {}, y: {}", x, y);
                        let ascii_display_x_offset = ascii_display_x_offset!();
                        let col_x;
                        let col_y = y / i64::from(row_height);
                        if x < ascii_display_x_offset {
                            col_x = x / i64::from(col_width);
                            per_msg!("col_x: {}, col_y: {}", col_x, col_y);
                        } else {
                            let x_rel = x - ascii_display_x_offset;
                            col_x = x_rel / i64::from(col_width / 2);
                        }
                        let new_cursor = usize::try_from(col_y).unwrap_or(0) * cols
                            + usize::try_from(col_x).unwrap_or(0);
                        cursor = starting_offset + new_cursor;
                    }
                }
                _ => {}
            }
        }
        if interact_mode == InteractMode::View && !input.key_down(Key::LControl) {
            let spd = if input.key_down(Key::LShift) {
                scroll_speed * 4
            } else {
                scroll_speed
            };
            if input.key_down(Key::Left) {
                view_x -= spd;
            } else if input.key_down(Key::Right) {
                view_x += spd;
            }
            if input.key_down(Key::Up) {
                view_y -= spd;
            } else if input.key_down(Key::Down) {
                view_y += spd;
            }
        }
        let cursor_changed = cursor != cursor_prev_frame;
        if cursor_changed {
            u8_buf = data[cursor].to_string();
        }
        // endregion
        w.clear(Color::BLACK);
        let mut rs = RenderStates::default();
        vertices.clear();
        sf_egui.do_frame(|ctx| {
            Window::new("Debug")
                .open(&mut show_debug_panel)
                .show(ctx, |ui| {
                    // region: debug panel
                    inspect! {
                        ui,
                        rows,
                        cols,
                        max_visible_cols,
                        starting_offset,
                        cursor,
                        edit_target,
                        row_height,
                        col_width,
                        view_x,
                        view_y,
                        scroll_speed
                    }
                    ui.separator();
                    ui.heading("More Debug");
                    for info in IMMEDIATE.lock().unwrap().iter() {
                        if let Info::Msg(msg) = info {
                            ui.label(msg);
                        }
                    }
                    gamedebug_core::clear_immediates();
                    ui.separator();
                    for PerEntry { frame, info } in PERSISTENT.lock().unwrap().iter() {
                        if let Info::Msg(msg) = info {
                            ui.label(format!("{}: {}", frame, msg));
                        }
                    }
                    // endregion
                });
            enum CursorViewStatus {
                Inside,
                Before,
                After,
            }
            // region: find window
            macro cursor_view_status() {
                if cursor < starting_offset {
                    CursorViewStatus::Before
                } else if cursor > starting_offset + rows * cols {
                    CursorViewStatus::After
                } else {
                    CursorViewStatus::Inside
                }
            }
            macro search_focus($off:expr) {
                // Focus the search result in the hex view
                cursor = $off;
                match cursor_view_status!() {
                    CursorViewStatus::Before => {
                        starting_offset = $off.saturating_sub((rows - 1) * (cols - 1))
                    }
                    CursorViewStatus::After => starting_offset = $off - (rows + cols),
                    CursorViewStatus::Inside => {}
                }
            }
            Window::new("Find")
                .open(&mut find_dialog.open)
                .show(ctx, |ui| {
                    if ui.text_edit_singleline(&mut find_dialog.input).lost_focus()
                        && ui.input().key_pressed(egui::Key::Enter)
                    {
                        let needle = find_dialog.input.parse().unwrap();
                        find_dialog.result_offsets.clear();
                        for (offset, &byte) in data.iter().enumerate() {
                            if byte == needle {
                                find_dialog.result_offsets.push(offset);
                            }
                        }
                        if let Some(&off) = find_dialog.result_offsets.first() {
                            search_focus!(off);
                        }
                    }
                    ScrollArea::vertical().max_height(480.).show(ui, |ui| {
                        for (i, &off) in find_dialog.result_offsets.iter().enumerate() {
                            let re = ui
                                .selectable_label(find_dialog.result_cursor == i, off.to_string());
                            if let Some(scroll_off) = find_dialog.scroll_to && scroll_off == i {
                            re.scroll_to_me(None);
                            find_dialog.scroll_to = None;
                        }
                            if re.clicked() {
                                search_focus!(off);
                                find_dialog.result_cursor = i;
                            }
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.set_enabled(!find_dialog.result_offsets.is_empty());
                        if (ui.button("Previous (P)").clicked()
                            || ui.input().key_pressed(egui::Key::P))
                            && find_dialog.result_cursor > 0
                        {
                            find_dialog.result_cursor -= 1;
                            let off = find_dialog.result_offsets[find_dialog.result_cursor];
                            search_focus!(off);
                            find_dialog.scroll_to = Some(find_dialog.result_cursor);
                        }
                        ui.label((find_dialog.result_cursor + 1).to_string());
                        if (ui.button("Next (N)").clicked() || ui.input().key_pressed(egui::Key::N))
                            && find_dialog.result_cursor + 1 < find_dialog.result_offsets.len()
                        {
                            find_dialog.result_cursor += 1;
                            let off = find_dialog.result_offsets[find_dialog.result_cursor];
                            search_focus!(off);
                            find_dialog.scroll_to = Some(find_dialog.result_cursor);
                        }
                        ui.label(format!("{} results", find_dialog.result_offsets.len()));
                    });
                });
            // endregion
            // region: top panel
            TopBottomPanel::top("top_panel").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let begin_text = match select_begin {
                        Some(begin) => begin.to_string(),
                        None => "-".to_owned(),
                    };
                    ui.label(format!("Select begin: {}", begin_text));
                    if ui.button("set").clicked() {
                        match &mut selection {
                            Some(sel) => sel.begin = cursor,
                            None => select_begin = Some(cursor),
                        }
                    }
                    let end_text = match selection {
                        Some(sel) => sel.end.to_string(),
                        None => "-".to_owned(),
                    };
                    ui.label(format!("end: {}", end_text));
                    if ui.button("set").clicked() {
                        match select_begin {
                            Some(begin) => match &mut selection {
                                None => selection = Some(Region { begin, end: cursor }),
                                Some(sel) => sel.end = cursor,
                            },
                            None => {}
                        }
                    }
                    if ui.button("deselect").clicked() {
                        selection = None;
                    }
                    ui.text_edit_singleline(&mut fill_text);
                    if ui.button("fill").clicked() {
                        if let Some(sel) = selection {
                            let values: Result<Vec<u8>, _> = fill_text
                                .split(' ')
                                .map(|token| u8::from_str_radix(token, 16))
                                .collect();
                            match values {
                                Ok(values) => {
                                    data[sel.begin..=sel.end].pattern_fill(&values);
                                    dirty = true;
                                }
                                Err(e) => {
                                    per_msg!("Fill parse error: {}", e);
                                }
                            }
                        }
                    }
                });
            });
            // endregion
            // region: bottom panel
            TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .selectable_label(interact_mode == InteractMode::View, "View (F1)")
                        .clicked()
                    {
                        interact_mode = InteractMode::View;
                    }
                    if ui
                        .selectable_label(interact_mode == InteractMode::Edit, "Edit (F2)")
                        .clicked()
                    {
                        interact_mode = InteractMode::Edit;
                    }
                    ui.separator();
                    match interact_mode {
                        InteractMode::View => {
                            ui.label(format!("offset: {}", starting_offset));
                            ui.label(format!("columns: {}", cols));
                        }
                        InteractMode::Edit => {
                            ui.label(format!("cursor: {}", cursor));
                            ui.separator();
                            ui.label("u8");
                            if ui
                                .add(TextEdit::singleline(&mut u8_buf).desired_width(28.0))
                                .lost_focus()
                                && ui.input().key_pressed(egui::Key::Enter)
                            {
                                data[cursor] = u8_buf.parse().unwrap();
                                dirty = true;
                            }
                        }
                    }
                    ui.with_layout(Layout::right_to_left(), |ui| {
                        ui.checkbox(&mut show_debug_panel, "debug (F12)");
                        ui.checkbox(&mut colorize, "color");
                        ui.checkbox(&mut show_text, "text");
                        ui.separator();
                        if ui
                            .add_enabled(dirty, Button::new("Reload (ctrl+R)"))
                            .clicked()
                        {
                            reload!();
                        }
                        if ui
                            .add_enabled(dirty, Button::new("Save (ctrl+S)"))
                            .clicked()
                        {
                            save!();
                        }
                        ui.separator();
                        if ui.button("Restore").clicked() {
                            std::fs::copy(&backup_path, &path).unwrap();
                            reload!();
                        }
                        if ui.button("Backup").clicked() {
                            std::fs::copy(&path, &backup_path).unwrap();
                        }
                    })
                })
            });
            // endregion
        });
        // region: hex display
        // The offset for the hex display imposed by the view
        let view_idx_off_x: usize = view_x.try_into().unwrap_or(0) / col_width as usize;
        let view_idx_off_y: usize = view_y.try_into().unwrap_or(0) / row_height as usize;
        let view_idx_off = view_idx_off_y * cols + view_idx_off_x;
        // The ascii view has a different offset indexing
        imm_msg!(view_idx_off_x);
        imm_msg!(view_idx_off_y);
        imm_msg!(view_idx_off);
        let mut idx = starting_offset + view_idx_off;
        let mut rows_rendered: u32 = 0;
        let mut cols_rendered: u32 = 0;
        'display: for y in 0..rows {
            for x in 0..cols {
                if x == max_visible_cols || x >= cols.saturating_sub(view_idx_off_x) {
                    idx += cols - x;
                    break;
                }
                if idx >= data.len() {
                    break 'display;
                }
                let pix_x = (x + view_idx_off_x) as f32 * f32::from(col_width) - view_x as f32;
                let pix_y = (y + view_idx_off_y) as f32 * f32::from(row_height) - view_y as f32;
                let byte = data[idx];
                let selected = match selection {
                    Some(sel) => (sel.begin..=sel.end).contains(&idx),
                    None => false,
                };
                if selected || (find_dialog.open && find_dialog.result_offsets.contains(&idx)) {
                    let mut rs = RectangleShape::from_rect(Rect::new(
                        pix_x,
                        pix_y,
                        col_width as f32,
                        row_height as f32,
                    ));
                    rs.set_fill_color(Color::rgb(150, 150, 150));
                    if cursor == idx {
                        rs.set_outline_color(Color::WHITE);
                        rs.set_outline_thickness(-2.0);
                    }
                    w.draw(&rs);
                }
                if idx == cursor {
                    let extra_x = if hex_edit_half_digit.is_none() {
                        0
                    } else {
                        col_width / 2
                    };
                    draw_cursor(
                        pix_x + extra_x as f32,
                        pix_y,
                        &mut w,
                        edit_target == EditTarget::Hex && interact_mode == InteractMode::Edit,
                    );
                }
                let [mut g1, g2] = hex_conv::byte_to_hex_digits(byte);
                if let Some(half) = hex_edit_half_digit && cursor == idx {
                    g1 = half.to_ascii_uppercase();
                }
                let [r, g, b] = rgb_from_hsv((byte as f32 / 255.0, 1.0, 1.0));
                let c = if colorize {
                    Color::rgb((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
                } else {
                    Color::WHITE
                };
                draw_glyph(&f, &mut vertices, pix_x, pix_y, g1 as u32, c);
                draw_glyph(&f, &mut vertices, pix_x + 11.0, pix_y, g2 as u32, c);
                idx += 1;
                cols_rendered += 1;
            }
            rows_rendered += 1;
        }
        imm_msg!(rows_rendered);
        cols_rendered = cols_rendered.checked_div(rows_rendered).unwrap_or(0);
        imm_msg!(cols_rendered);
        // endregion
        // region: ascii display
        // The offset for the ascii display imposed by the view
        let ascii_display_x_offset = ascii_display_x_offset!();
        imm_msg!(ascii_display_x_offset);
        let view_idx_off_x: usize = view_x
            .saturating_sub(ascii_display_x_offset)
            .try_into()
            .unwrap_or(0)
            / col_width as usize;
        //let view_idx_off_y: usize = view_y.try_into().unwrap_or(0) / row_height as usize;
        let view_idx_off = view_idx_off_y * cols + view_idx_off_x;
        imm_msg!("ascii");
        imm_msg!(view_idx_off_x);
        //imm_msg!(view_idx_off_y);
        imm_msg!(view_idx_off);
        let mut ascii_rows_rendered: u32 = 0;
        let mut ascii_cols_rendered: u32 = 0;
        if show_text {
            idx = starting_offset + view_idx_off;
            imm_msg!(idx);
            'asciidisplay: for y in 0..rows {
                for x in 0..cols {
                    if x == max_visible_cols * 2 || x >= cols.saturating_sub(view_idx_off_x) {
                        idx += cols - x;
                        break;
                    }
                    if idx >= data.len() {
                        break 'asciidisplay;
                    }
                    let pix_x =
                        (x + cols * 2 + 1) as f32 * f32::from(col_width / 2) - view_x as f32;
                    //let pix_y = y as f32 * f32::from(row_height) - view_y as f32;
                    let pix_y = (y + view_idx_off_y) as f32 * f32::from(row_height) - view_y as f32;
                    let byte = data[idx];
                    let [r, g, b] = rgb_from_hsv((byte as f32 / 255.0, 1.0, 1.0));
                    let c = if colorize {
                        Color::rgb((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
                    } else {
                        Color::WHITE
                    };
                    let selected = match selection {
                        Some(sel) => (sel.begin..=sel.end).contains(&idx),
                        None => false,
                    };
                    if selected || (find_dialog.open && find_dialog.result_offsets.contains(&idx)) {
                        let mut rs = RectangleShape::from_rect(Rect::new(
                            pix_x,
                            pix_y,
                            (col_width / 2) as f32,
                            row_height as f32,
                        ));
                        rs.set_fill_color(Color::rgb(150, 150, 150));
                        if cursor == idx {
                            rs.set_outline_color(Color::WHITE);
                            rs.set_outline_thickness(-2.0);
                        }
                        w.draw(&rs);
                    }
                    if idx == cursor {
                        draw_cursor(
                            pix_x,
                            pix_y,
                            &mut w,
                            edit_target == EditTarget::Text && interact_mode == InteractMode::Edit,
                        );
                    }
                    draw_glyph(&f, &mut vertices, pix_x, pix_y, byte as u32, c);
                    idx += 1;
                    ascii_cols_rendered += 1;
                }
                ascii_rows_rendered += 1;
            }
        }
        imm_msg!(ascii_rows_rendered);
        ascii_cols_rendered = ascii_cols_rendered
            .checked_div(ascii_rows_rendered)
            .unwrap_or(0);
        imm_msg!(ascii_cols_rendered);
        // endregion
        rs.set_texture(Some(f.texture(10)));
        w.draw_primitives(&vertices, PrimitiveType::QUADS, &rs);
        rs.set_texture(None);
        sf_egui.draw(&mut w, None);
        w.display();
        gamedebug_core::inc_frame();
        cursor_prev_frame = cursor;
    }
}

fn draw_cursor(x: f32, y: f32, w: &mut RenderWindow, active: bool) {
    let mut rs = RectangleShape::from_rect(Rect {
        left: x,
        top: y,
        width: 10.0,
        height: 10.0,
    });
    rs.set_fill_color(Color::TRANSPARENT);
    rs.set_outline_thickness(2.0);
    if active {
        rs.set_outline_color(Color::WHITE);
    } else {
        rs.set_outline_color(Color::rgb(150, 150, 150));
    }
    w.draw(&rs);
}

fn draw_glyph(font: &Font, vertices: &mut Vec<Vertex>, x: f32, y: f32, glyph: u32, color: Color) {
    let g = font.glyph(glyph, 10, false, 0.0);
    let r = g.texture_rect();
    vertices.push(Vertex {
        position: Vector2::new(x, y),
        color,
        tex_coords: Vector2::new(r.left as f32, r.top as f32),
    });
    vertices.push(Vertex {
        position: Vector2::new(x, y + 10.0),
        color,
        tex_coords: Vector2::new(r.left as f32, (r.top + r.height) as f32),
    });
    vertices.push(Vertex {
        position: Vector2::new(x + 10.0, y + 10.0),
        color,
        tex_coords: Vector2::new((r.left + r.width) as f32, (r.top + r.height) as f32),
    });
    vertices.push(Vertex {
        position: Vector2::new(x + 10.0, y),
        color,
        tex_coords: Vector2::new((r.left + r.width) as f32, r.top as f32),
    });
}
