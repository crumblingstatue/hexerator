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
    hex_conv,
    region::Region,
    ui::Ui,
    view::ViewKind,
};

use super::View;

pub fn draw_view(
    view: &View,
    app: &App,
    vertex_buffer: &mut Vec<Vertex>,
    mut drawfn: impl FnMut(&mut Vec<Vertex>, f32, f32, u8, usize, Color),
) {
    // Protect against infinite loop lock up when scrolling horizontally out of view
    if view.scroll_offset.pix_xoff <= -view.viewport_rect.w {
        return;
    }
    let mut idx = app.perspective.region.begin;
    let start_row: usize = view.scroll_offset.row;
    idx += start_row * app.perspective.cols;
    imm_msg!(view.rows());
    let orig = start_row..=start_row + view.rows();
    let (row_range, pix_yoff) = if app.perspective.flip_row_order {
        (Either::Left(orig.rev()), -view.scroll_offset.pix_yoff)
    } else {
        (Either::Right(orig), view.scroll_offset.pix_yoff)
    };
    'rows: for row in row_range {
        let y = row as f32 * f32::from(view.row_h);
        let viewport_y = (view.viewport_rect.y as f32 + y)
            - ((view.scroll_offset.row as f32 * view.row_h as f32) + pix_yoff as f32);
        let start_col = view.scroll_offset.col;
        if start_col >= app.perspective.cols {
            break;
        }
        idx += start_col;
        for col in start_col..app.perspective.cols {
            let x = col as f32 * f32::from(view.col_w);
            let viewport_x = (view.viewport_rect.x as f32 + x)
                - ((view.scroll_offset.col as f32 * view.col_w as f32)
                    + view.scroll_offset.pix_xoff as f32);
            if viewport_x > (view.viewport_rect.x + view.viewport_rect.w) as f32 {
                idx += app.perspective.cols - col;
                break;
            }
            if viewport_y > (view.viewport_rect.y + view.viewport_rect.h) as f32
                && !app.perspective.flip_row_order
            {
                break 'rows;
            }
            match app.data.get(idx) {
                Some(&byte) => {
                    let c = app
                        .presentation
                        .color_method
                        .byte_color(byte, app.presentation.invert_color);
                    drawfn(vertex_buffer, viewport_x, viewport_y, byte, idx, c);
                    idx += 1;
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
        //app.scissor_views = false;
        vertex_buffer.clear();
        let mut rs = RenderStates::default();
        match self.kind {
            ViewKind::Hex => {
                draw_view(
                    self,
                    app,
                    vertex_buffer,
                    |vertex_buffer, x, y, byte, idx, c| {
                        if selected_or_find_result_contains(app.selection, idx, &app.ui) {
                            draw_rect(
                                vertex_buffer,
                                x,
                                y,
                                self.col_w as f32,
                                self.row_h as f32,
                                app.presentation.sel_color,
                            )
                        }
                        let [mut d1, d2] = hex_conv::byte_to_hex_digits(byte);
                        if let Some(half) = app.edit_state.hex_edit_half_digit && app.edit_state.cursor == idx {
                            d1 = half.to_ascii_uppercase();
                        }
                        draw_glyph(
                            font,
                            app.layout.font_size.into(),
                            vertex_buffer,
                            x,
                            y,
                            d1.into(),
                            c,
                        );
                        draw_glyph(
                            font,
                            app.layout.font_size.into(),
                            vertex_buffer,
                            x + (self.col_w / 2) as f32 - 4.0,
                            y,
                            d2.into(),
                            c,
                        );
                        let extra_x = if app.edit_state.hex_edit_half_digit.is_none() {
                            0
                        } else {
                            self.col_w / 2 - 4
                        };
                        if idx == app.edit_state.cursor {
                            draw_cursor(
                                x + extra_x as f32,
                                y,
                                vertex_buffer,
                                true,
                                app.cursor_flash_timer(),
                                &app.presentation,
                            );
                        }
                    },
                );
                rs.set_texture(Some(font.texture(app.layout.font_size.into())));
            }
            ViewKind::Ascii => {
                draw_view(
                    self,
                    app,
                    vertex_buffer,
                    |vertex_buffer, x, y, byte, idx, c| {
                        if selected_or_find_result_contains(app.selection, idx, &app.ui) {
                            draw_rect(
                                vertex_buffer,
                                x,
                                y,
                                self.col_w as f32,
                                self.row_h as f32,
                                app.presentation.sel_color,
                            )
                        }
                        let glyph = match byte {
                            0x00 => '∅' as u32,
                            0x0A => '⏎' as u32,
                            0x0D => '⇤' as u32,
                            0x20 => '␣' as u32,
                            0xFF => '■' as u32,
                            _ => byte as u32,
                        };
                        draw_glyph(
                            font,
                            app.layout.font_size.into(),
                            vertex_buffer,
                            x,
                            y,
                            glyph,
                            c,
                        );
                        if idx == app.edit_state.cursor {
                            draw_cursor(
                                x,
                                y,
                                vertex_buffer,
                                true,
                                app.cursor_flash_timer(),
                                &app.presentation,
                            );
                        }
                    },
                );
                rs.set_texture(Some(font.texture(app.layout.font_size.into())));
            }
            ViewKind::Block => {
                draw_view(
                    self,
                    app,
                    vertex_buffer,
                    |vertex_buffer, x, y, _byte, idx, mut c| {
                        if selected_or_find_result_contains(app.selection, idx, &app.ui) {
                            c = invert_color(c);
                        }
                        draw_rect(vertex_buffer, x, y, self.col_w as f32, self.row_h as f32, c);
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
            -1.0,
        );
        if app.scissor_views {
            unsafe {
                glu_sys::glEnable(glu_sys::GL_SCISSOR_TEST);
                let y = window.size().y as GLint
                    - GLint::from(self.viewport_rect.y + self.viewport_rect.h);
                glu_sys::glScissor(
                    self.viewport_rect.x.into(),
                    y,
                    self.viewport_rect.w.into(),
                    self.viewport_rect.h.into(),
                );
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
