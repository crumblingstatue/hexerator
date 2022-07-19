use sfml::graphics::{Font, Vertex};

use crate::{
    app::App,
    hex_conv,
    view::{draw_cursor, draw_glyph},
};

use super::View;

pub fn hex(lens: &View, app: &mut App, font: &Font, vertex_buffer: &mut Vec<Vertex>) {
    let mut idx = app.view.region.begin;
    let start_row = app.view_y.try_into().unwrap_or(0) / lens.row_h as usize;
    idx += start_row * app.view.cols;
    'rows: for row in start_row.. {
        let y = row as f32 * f32::from(lens.row_h);
        let yy = (lens.viewport_rect.y as f32 + y) - app.view_y as f32;
        let start_col = app.view_x.try_into().unwrap_or(0) / lens.col_w as usize;
        if start_col >= app.view.cols {
            break;
        }
        idx += start_col;
        for col in start_col..app.view.cols {
            let x = col as f32 * f32::from(lens.col_w);
            let xx = (lens.viewport_rect.x as f32 + x) - app.view_x as f32;
            if xx > (lens.viewport_rect.x + lens.viewport_rect.w) as f32 {
                idx += app.view.cols - col;
                break;
            }
            if yy > (lens.viewport_rect.y + lens.viewport_rect.h) as f32 || idx >= app.data.len() {
                break 'rows;
            }
            let byte = app.data[idx];
            let c = app
                .presentation
                .color_method
                .byte_color(byte, app.presentation.invert_color);
            let [d1, d2] = hex_conv::byte_to_hex_digits(byte);
            draw_glyph(
                font,
                app.layout.font_size.into(),
                vertex_buffer,
                xx,
                yy,
                d1.into(),
                c,
            );
            draw_glyph(
                font,
                app.layout.font_size.into(),
                vertex_buffer,
                xx + (lens.col_w / 2) as f32 - 4.0,
                yy,
                d2.into(),
                c,
            );
            if idx == app.edit_state.cursor {
                draw_cursor(
                    xx,
                    yy,
                    vertex_buffer,
                    true,
                    app.cursor_flash_timer(),
                    &app.presentation,
                );
            }
            idx += 1;
        }
    }
}
