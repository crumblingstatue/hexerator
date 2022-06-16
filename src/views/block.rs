use gamedebug_core::imm_msg;
use sfml::graphics::{RenderTarget, RenderWindow, Sprite, Texture, Transformable};

use crate::{app::App, views::draw_cursor, EditTarget, InteractMode};

pub fn block(app: &mut App, view_idx_off_y: usize, window: &mut RenderWindow) {
    let view_offset = app.block_display_x_offset();
    imm_msg!(view_offset);
    let view_idx_off_x: usize = app
        .view_x
        .saturating_sub(view_offset)
        .try_into()
        .unwrap_or(0)
        / app.col_width as usize;
    let view_idx_off = view_idx_off_y * app.view.cols + view_idx_off_x;
    imm_msg!("block");
    imm_msg!(view_idx_off_x);
    imm_msg!(view_idx_off);
    let mut block_rows_rendered: u32 = 0;
    let mut block_cols_rendered: u32 = 0;
    let mut idx = app.view.start_offset + view_idx_off;
    imm_msg!(idx);
    let rows = 240;
    let mut pixels = vec![255; app.view.cols * rows * 4];
    let max_visible_cols = 300;
    'display: for y in 0..rows {
        for x in 0..app.view.cols {
            if x == max_visible_cols * 2 || x >= app.view.cols.saturating_sub(view_idx_off_x) {
                idx += app.view.cols - x;
                break;
            }
            if idx >= app.data.len() {
                break 'display;
            }
            let pix_x =
                (x + app.view.cols * 2 + 1) as f32 * f32::from(app.block_size) - app.view_x as f32;
            let pix_y = (y + view_idx_off_y) as f32 * f32::from(app.block_size) - app.view_y as f32;
            let byte = app.data[idx];
            let c = app
                .presentation
                .color_method
                .byte_color(byte, app.presentation.invert_color);
            let idxx = (y * app.view.cols + x) * 4;
            pixels[idxx] = c.red();
            pixels[idxx + 1] = c.green();
            pixels[idxx + 2] = c.blue();
            let selected = match app.selection {
                Some(sel) => (sel.begin..=sel.end).contains(&idx),
                None => false,
            };
            if selected
                || (app.ui.find_dialog.open && app.ui.find_dialog.result_offsets.contains(&idx))
            {
            }
            if idx == app.edit_state.cursor {
                draw_cursor(
                    pix_x,
                    pix_y,
                    window,
                    app.edit_target == EditTarget::Text && app.interact_mode == InteractMode::Edit,
                    app.cursor_flash_timer(),
                );
            }
            idx += 1;
            block_cols_rendered += 1;
        }
        block_rows_rendered += 1;
    }
    let mut t = Texture::new().unwrap();
    let _ = t.create(app.view.cols as _, rows as _);
    unsafe {
        t.update_from_pixels(&pixels, app.view.cols as _, rows as _, 0, 0);
    }
    let mut s = Sprite::with_texture(&t);
    s.set_position((-app.view_x as _, app.top_gap as f32 - app.view_y as f32));
    s.set_scale((app.block_size as _, app.block_size as _));
    window.draw(&s);
    imm_msg!(block_rows_rendered);
    block_cols_rendered = block_cols_rendered
        .checked_div(block_rows_rendered)
        .unwrap_or(0);
    imm_msg!(block_cols_rendered);
}
