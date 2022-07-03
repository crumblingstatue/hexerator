use gamedebug_core::imm_msg;
use sfml::graphics::{RenderTarget, RenderWindow, Sprite, Texture, Transformable, Vertex};

use crate::{
    app::{edit_target::EditTarget, interact_mode::InteractMode, App},
    lens::draw_cursor,
};

use super::{draw_rect, Lens};

pub fn block(
    lens: &Lens,
    app: &mut App,
    window: &mut RenderWindow,
    vertex_buffer: &mut Vec<Vertex>,
) {
    let mut idx = app.view.region.begin;
    let start_row = app.view_y.try_into().unwrap_or(0) / lens.row_h as usize;
    idx += start_row * app.view.cols;
    'rows: for row in start_row..app.view.rows {
        let y = row as f32 * f32::from(lens.row_h);
        let yy = (lens.y as f32 + y) - app.view_y as f32;
        let start_col = app.view_x.try_into().unwrap_or(0) / lens.col_w as usize;
        if start_col >= app.view.cols {
            break;
        }
        idx += start_col;
        for col in start_col..app.view.cols {
            let x = col as f32 * f32::from(lens.col_w);
            let xx = (lens.x as f32 + x) - app.view_x as f32;
            if xx > (lens.x + lens.w) as f32 {
                idx += app.view.cols - col;
                break;
            }
            if yy > (lens.y + lens.h) as f32 || idx >= app.data.len() {
                break 'rows;
            }
            let byte = app.data[idx];
            let c = app
                .presentation
                .color_method
                .byte_color(byte, app.presentation.invert_color);
            draw_rect(
                vertex_buffer,
                xx,
                yy,
                lens.col_w as f32,
                lens.row_h as f32,
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
