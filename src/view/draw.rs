use either::Either;
use gamedebug_core::imm_msg;
use glu_sys::GLint;
use sfml::{
    graphics::{Color, Font, PrimitiveType, RenderStates, RenderTarget, RenderWindow, Vertex},
    system::Vector2,
};

use crate::{
    app::{presentation::Presentation, App},
    color::invert_color,
    dec_conv, hex_conv,
    region::Region,
    ui::Ui,
    view::ViewKind,
};

use super::View;

pub fn draw_view(
    view: &View,
    app: &App,
    vertex_buffer: &mut Vec<Vertex>,
    mut drawfn: impl FnMut(&mut Vec<Vertex>, f32, f32, &[u8], usize, Color),
) {
    // Protect against infinite loop lock up when scrolling horizontally out of view
    if view.scroll_offset.pix_xoff <= -view.viewport_rect.w {
        return;
    }
    let mut idx = app.perspective.region.begin;
    let start_row: usize = view.scroll_offset.row;
    imm_msg!(start_row);
    idx += start_row * (app.perspective.cols * usize::from(view.bytes_per_block));
    imm_msg!(view.rows());
    #[expect(
        clippy::cast_sign_loss,
        reason = "rows() returning negative is a bug, should be positive."
    )]
    let orig = start_row..=start_row + view.rows() as usize;
    let (row_range, pix_yoff) = if app.perspective.flip_row_order {
        (Either::Left(orig.rev()), -view.scroll_offset.pix_yoff)
    } else {
        (Either::Right(orig), view.scroll_offset.pix_yoff)
    };
    'rows: for row in row_range {
        let y = row * usize::from(view.row_h);
        let viewport_y = (i64::from(view.viewport_rect.y) + y as i64)
            - ((view.scroll_offset.row as i64 * i64::from(view.row_h)) + i64::from(pix_yoff));
        let start_col = view.scroll_offset.col;
        if start_col >= app.perspective.cols {
            break;
        }
        idx += start_col * usize::from(view.bytes_per_block);
        for col in start_col..app.perspective.cols {
            let x = col * usize::from(view.col_w);
            let viewport_x = (i64::from(view.viewport_rect.x) + x as i64)
                - ((view.scroll_offset.col as i64 * i64::from(view.col_w))
                    + i64::from(view.scroll_offset.pix_xoff));
            if viewport_x > i64::from(view.viewport_rect.x + view.viewport_rect.w) {
                idx += (app.perspective.cols - col) * usize::from(view.bytes_per_block);
                break;
            }
            if viewport_y > i64::from(view.viewport_rect.y + view.viewport_rect.h)
                && !app.perspective.flip_row_order
            {
                break 'rows;
            }
            match app.data.get(idx..idx + view.bytes_per_block as usize) {
                Some(data) => {
                    let c = app
                        .presentation
                        .color_method
                        .byte_color(data[0], app.presentation.invert_color);
                    #[expect(
                        clippy::cast_precision_loss,
                        reason = "At this point, the viewport coordinates should be small enough to fit in viewport"
                    )]
                    drawfn(
                        vertex_buffer,
                        viewport_x as f32,
                        viewport_y as f32,
                        data,
                        idx,
                        c,
                    );
                    idx += usize::from(view.bytes_per_block);
                }
                None => {
                    if !app.perspective.flip_row_order {
                        break 'rows;
                    }
                }
            }
        }
    }
}

fn draw_cursor(
    x: f32,
    y: f32,
    vertices: &mut Vec<Vertex>,
    active: bool,
    flash_timer: Option<u32>,
    presentation: &Presentation,
) {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "Deliberate color modulation based on timer value."
    )]
    let color = if active {
        match flash_timer {
            Some(timer) => Color::rgb(timer as u8, timer as u8, timer as u8),
            None => presentation.cursor_active_color,
        }
    } else {
        match flash_timer {
            Some(timer) => Color::rgb(timer as u8, timer as u8, timer as u8),
            None => presentation.cursor_color,
        }
    };
    draw_rect_outline(vertices, x, y, 10.0, 10.0, color, 2.0);
}

#[expect(
    clippy::cast_precision_loss,
    reason = "These casts deal with texture rect coords.
              These aren't expected to be larger than what fits into f32"
)]
fn draw_glyph(
    font: &Font,
    font_size: u32,
    vertices: &mut Vec<Vertex>,
    mut x: f32,
    mut y: f32,
    glyph: u32,
    color: Color,
) {
    let glyph = font.glyph(glyph, font_size, false, 0.0);
    let bounds = glyph.bounds();
    let baseline = 10.0; // TODO: Stupid assumption
    y += baseline;
    x += bounds.left;
    y += bounds.top;
    let texture_rect = glyph.texture_rect();
    vertices.push(Vertex {
        position: Vector2::new(x, y),
        color,
        tex_coords: texture_rect.position().as_other(),
    });
    vertices.push(Vertex {
        position: Vector2::new(x, y + bounds.height),
        color,
        tex_coords: Vector2::new(
            texture_rect.left as f32,
            (texture_rect.top + texture_rect.height) as f32,
        ),
    });
    vertices.push(Vertex {
        position: Vector2::new(x + bounds.width, y + bounds.height),
        color,
        tex_coords: Vector2::new(
            (texture_rect.left + texture_rect.width) as f32,
            (texture_rect.top + texture_rect.height) as f32,
        ),
    });
    vertices.push(Vertex {
        position: Vector2::new(x + bounds.width, y),
        color,
        tex_coords: Vector2::new(
            (texture_rect.left + texture_rect.width) as f32,
            texture_rect.top as f32,
        ),
    });
}

fn draw_rect(vertices: &mut Vec<Vertex>, x: f32, y: f32, w: f32, h: f32, color: Color) {
    vertices.extend([
        Vertex {
            position: Vector2::new(x, y),
            color,
            tex_coords: Vector2::default(),
        },
        Vertex {
            position: Vector2::new(x, y + h),
            color,
            tex_coords: Vector2::default(),
        },
        Vertex {
            position: Vector2::new(x + w, y + h),
            color,
            tex_coords: Vector2::default(),
        },
        Vertex {
            position: Vector2::new(x + w, y),
            color,
            tex_coords: Vector2::default(),
        },
    ]);
}

fn draw_rect_outline(
    vertices: &mut Vec<Vertex>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    color: Color,
    thickness: f32,
) {
    // top
    draw_rect(
        vertices,
        x - thickness,
        y - thickness,
        w + thickness,
        thickness,
        color,
    );
    // right
    draw_rect(
        vertices,
        x + w,
        y - thickness,
        thickness,
        h + thickness,
        color,
    );
    // bottom
    draw_rect(
        vertices,
        x - thickness,
        y + h,
        w + thickness * 2.0,
        thickness,
        color,
    );
    // left
    draw_rect(
        vertices,
        x - thickness,
        y - thickness,
        thickness,
        h + thickness,
        color,
    );
}

impl View {
    pub fn draw(
        &self,
        key: usize,
        app: &mut App,
        window: &mut RenderWindow,
        vertex_buffer: &mut Vec<Vertex>,
        font: &Font,
    ) {
        if !self.active {
            return;
        }
        //app.scissor_views = false;
        vertex_buffer.clear();
        let mut rs = RenderStates::default();
        match self.kind {
            ViewKind::Hex => {
                draw_view(
                    self,
                    app,
                    vertex_buffer,
                    |vertex_buffer, x, y, data, idx, c| {
                        if selected_or_find_result_contains(
                            App::selection(&app.select_a, &app.select_b),
                            idx,
                            &app.ui,
                        ) {
                            draw_rect(
                                vertex_buffer,
                                x,
                                y,
                                f32::from(self.col_w),
                                f32::from(self.row_h),
                                app.presentation.sel_color,
                            )
                        }
                        let mut gx = x;
                        for (i, mut d) in hex_conv::byte_to_hex_digits(data[0])
                            .into_iter()
                            .enumerate()
                        {
                            if idx == app.edit_state.cursor && self.edit_buf.dirty {
                                d = self.edit_buf.buf[i];
                            }
                            draw_glyph(
                                font,
                                self.font_size.into(),
                                vertex_buffer,
                                gx,
                                y,
                                d.into(),
                                c,
                            );
                            gx += f32::from(self.font_size - 4);
                        }
                        let extra_x = self.edit_buf.cursor * u16::from(self.font_size - 4);
                        if idx == app.edit_state.cursor {
                            draw_cursor(
                                x + f32::from(extra_x),
                                y,
                                vertex_buffer,
                                app.focused_view == Some(key),
                                app.cursor_flash_timer(),
                                &app.presentation,
                            );
                        }
                    },
                );
                rs.set_texture(Some(font.texture(self.font_size.into())));
            }
            ViewKind::Dec => {
                draw_view(
                    self,
                    app,
                    vertex_buffer,
                    |vertex_buffer, x, y, data, idx, c| {
                        if selected_or_find_result_contains(
                            App::selection(&app.select_a, &app.select_b),
                            idx,
                            &app.ui,
                        ) {
                            draw_rect(
                                vertex_buffer,
                                x,
                                y,
                                f32::from(self.col_w),
                                f32::from(self.row_h),
                                app.presentation.sel_color,
                            )
                        }
                        let mut gx = x;
                        for (i, mut d) in dec_conv::byte_to_dec_digits(data[0])
                            .into_iter()
                            .enumerate()
                        {
                            if idx == app.edit_state.cursor && self.edit_buf.dirty {
                                d = self.edit_buf.buf[i];
                            }
                            draw_glyph(
                                font,
                                self.font_size.into(),
                                vertex_buffer,
                                gx,
                                y,
                                d.into(),
                                c,
                            );
                            gx += f32::from(self.font_size - 4);
                        }
                        let extra_x = self.edit_buf.cursor * u16::from(self.font_size - 4);
                        if idx == app.edit_state.cursor {
                            draw_cursor(
                                x + f32::from(extra_x),
                                y,
                                vertex_buffer,
                                app.focused_view == Some(key),
                                app.cursor_flash_timer(),
                                &app.presentation,
                            );
                        }
                    },
                );
                rs.set_texture(Some(font.texture(self.font_size.into())));
            }
            ViewKind::Text => {
                draw_view(
                    self,
                    app,
                    vertex_buffer,
                    |vertex_buffer, x, y, data, idx, c| {
                        if selected_or_find_result_contains(
                            App::selection(&app.select_a, &app.select_b),
                            idx,
                            &app.ui,
                        ) {
                            draw_rect(
                                vertex_buffer,
                                x,
                                y,
                                f32::from(self.col_w),
                                f32::from(self.row_h),
                                app.presentation.sel_color,
                            )
                        }
                        let raw_data = match self.text_kind {
                            crate::view::TextKind::Ascii => u32::from(data[0]),
                            crate::view::TextKind::Utf16Le => {
                                u32::from(u16::from_le_bytes([data[0], data[1]]))
                            }
                            crate::view::TextKind::Utf16Be => {
                                u32::from(u16::from_be_bytes([data[0], data[1]]))
                            }
                        };
                        let glyph = match raw_data {
                            0x00 => '∅' as u32,
                            0x09 => '⇥' as u32,
                            0x0A => '⏎' as u32,
                            0x0D => '⇤' as u32,
                            0x20 => '␣' as u32,
                            0xFF => '■' as u32,
                            _ => raw_data,
                        };
                        draw_glyph(font, self.font_size.into(), vertex_buffer, x, y, glyph, c);
                        if idx == app.edit_state.cursor {
                            draw_cursor(
                                x,
                                y,
                                vertex_buffer,
                                app.focused_view == Some(key),
                                app.cursor_flash_timer(),
                                &app.presentation,
                            );
                        }
                    },
                );
                rs.set_texture(Some(font.texture(self.font_size.into())));
            }
            ViewKind::Block => {
                draw_view(
                    self,
                    app,
                    vertex_buffer,
                    |vertex_buffer, x, y, _byte, idx, mut c| {
                        if selected_or_find_result_contains(
                            App::selection(&app.select_a, &app.select_b),
                            idx,
                            &app.ui,
                        ) {
                            c = invert_color(c);
                        }
                        draw_rect(
                            vertex_buffer,
                            x,
                            y,
                            f32::from(self.col_w),
                            f32::from(self.row_h),
                            c,
                        );
                    },
                );
            }
        }
        draw_rect_outline(
            vertex_buffer,
            self.viewport_rect.x.into(),
            self.viewport_rect.y.into(),
            self.viewport_rect.w.into(),
            self.viewport_rect.h.into(),
            if Some(key) == app.focused_view {
                Color::rgb(255, 255, 150)
            } else {
                Color::rgb(120, 120, 150)
            },
            1.0,
        );
        if app.scissor_views {
            unsafe {
                glu_sys::glEnable(glu_sys::GL_SCISSOR_TEST);
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "Huge window sizes (>32000) are not supported."
                )]
                let vh = window.size().y as i16;
                let (x, y, w, h) = rect_to_gl_viewport(
                    self.viewport_rect.x - 1,
                    self.viewport_rect.y - 1,
                    self.viewport_rect.w + 2,
                    self.viewport_rect.h + 2,
                    vh,
                );
                glu_sys::glScissor(x, y, w, h);
            }
        }
        window.draw_primitives(vertex_buffer, PrimitiveType::QUADS, &rs);
        imm_msg!(vertex_buffer.len());
        if app.scissor_views {
            unsafe {
                glu_sys::glDisable(glu_sys::GL_SCISSOR_TEST);
            }
        }
    }
}

fn rect_to_gl_viewport(x: i16, y: i16, w: i16, h: i16, viewport_h: i16) -> (i32, i32, i32, i32) {
    (
        GLint::from(x),
        GLint::from(viewport_h - (y + h)),
        GLint::from(w),
        GLint::from(h),
    )
}

#[test]
fn test_rect_to_gl() {
    let vh = 1080;
    assert_eq!(rect_to_gl_viewport(0, 0, 0, 0, vh), (0, 1080, 0, 0));
    assert_eq!(
        rect_to_gl_viewport(100, 480, 300, 400, vh),
        (100, 200, 300, 400)
    );
}

fn selected_or_find_result_contains(
    app_selection: Option<Region>,
    idx: usize,
    app_ui: &Ui,
) -> bool {
    selected(app_selection, idx) || find_result_contains(app_ui, idx)
}

fn find_result_contains(app_ui: &Ui, idx: usize) -> bool {
    app_ui.find_dialog.open && app_ui.find_dialog.result_offsets.contains(&idx)
}

fn selected(app_selection: Option<Region>, idx: usize) -> bool {
    match app_selection {
        Some(sel) => (sel.begin..=sel.end).contains(&idx),
        None => false,
    }
}
