use gamedebug_core::imm_msg;
use sfml::graphics::{Font, Vertex};

use crate::{
    app::{edit_target::EditTarget, interact_mode::InteractMode, App},
    hex_conv,
    lens::{draw_cursor, draw_glyph},
};

pub fn hex(
    view_idx_off_y: usize,
    app: &mut App,
    view_idx_off_x: usize,
    font: &Font,
    vertex_buffer: &mut Vec<Vertex>,
) {
    let view_idx_off = view_idx_off_y * app.view.cols + view_idx_off_x;
    // The ascii view has a different offset indexing
    imm_msg!(view_idx_off_x);
    imm_msg!(view_idx_off_y);
    imm_msg!(view_idx_off);
    let mut idx = app.view.region.begin + view_idx_off;
    let mut rows_rendered: u32 = 0;
    let mut cols_rendered: u32 = 0;
    'display: for y in 0..app.view.rows {
        for x in 0..app.view.cols {
            if x == app.layout.max_visible_cols || x >= app.view.cols.saturating_sub(view_idx_off_x)
            {
                idx += app.view.cols - x;
                break;
            }
            if idx >= app.data.len() {
                break 'display;
            }
            let pix_x =
                (x + view_idx_off_x) as f32 * f32::from(app.layout.col_width) - app.view_x as f32;
            let pix_y =
                (y + view_idx_off_y) as f32 * f32::from(app.layout.row_height) - app.view_y as f32;
            let byte = app.data[idx];
            let selected = match app.selection {
                Some(sel) => (sel.begin..=sel.end).contains(&idx),
                None => false,
            };
            if selected
                || (app.ui.find_dialog.open && app.ui.find_dialog.result_offsets.contains(&idx))
            {
                super::draw_rect(
                    vertex_buffer,
                    pix_x,
                    pix_y,
                    app.layout.col_width as f32,
                    app.layout.row_height as f32,
                    app.presentation.sel_color,
                );
            }
            if idx == app.edit_state.cursor {
                let extra_x = if app.edit_state.hex_edit_half_digit.is_none() {
                    0
                } else {
                    app.layout.col_width / 2
                };
                draw_cursor(
                    pix_x + extra_x as f32,
                    pix_y,
                    vertex_buffer,
                    app.edit_target == EditTarget::Hex && app.interact_mode == InteractMode::Edit,
                    app.cursor_flash_timer(),
                    &app.presentation,
                );
            }
            let [mut g1, g2] = hex_conv::byte_to_hex_digits(byte);
            if let Some(half) = app.edit_state.hex_edit_half_digit && app.edit_state.cursor == idx {
                g1 = half.to_ascii_uppercase();
            }
            let c = app
                .presentation
                .color_method
                .byte_color(byte, app.presentation.invert_color);
            draw_glyph(
                font,
                app.layout.font_size,
                vertex_buffer,
                pix_x,
                pix_y,
                g1 as u32,
                c,
            );
            draw_glyph(
                font,
                app.layout.font_size,
                vertex_buffer,
                pix_x + 11.0,
                pix_y,
                g2 as u32,
                c,
            );
            idx += 1;
            cols_rendered += 1;
        }
        rows_rendered += 1;
    }
    imm_msg!(rows_rendered);
    cols_rendered = cols_rendered.checked_div(rows_rendered).unwrap_or(0);
    imm_msg!(cols_rendered);
}
