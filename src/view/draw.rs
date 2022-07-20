use gamedebug_core::imm_msg;
use glu_sys::GLint;
use sfml::{
    graphics::{Color, Font, PrimitiveType, RenderStates, RenderTarget, RenderWindow, Vertex},
    system::Vector2,
};

use crate::{
    app::{presentation::Presentation, App},
    hex_conv,
    view::ViewKind,
};

use super::View;

pub fn draw_view(
    view: &View,
    app: &App,
    vertex_buffer: &mut Vec<Vertex>,
    mut drawfn: impl FnMut(&mut Vec<Vertex>, f32, f32, u8, Color),
) {
    let view_x = view.scroll_offset.col_x;
    let view_y = view.scroll_offset.row_y;
    let mut idx = app.perspective.region.begin;
    let start_row = view_y.try_into().unwrap_or(0) / view.row_h as usize;
    idx += start_row * app.perspective.cols;
    'rows: for row in start_row.. {
        let y = row as f32 * f32::from(view.row_h);
        let yy = (view.viewport_rect.y as f32 + y) - view_y as f32;
        let start_col = view_x.try_into().unwrap_or(0) / view.col_w as usize;
        if start_col >= app.perspective.cols {
            break;
        }
        idx += start_col;
        for col in start_col..app.perspective.cols {
            let x = col as f32 * f32::from(view.col_w);
            let xx = (view.viewport_rect.x as f32 + x) - view_x as f32;
            if xx > (view.viewport_rect.x + view.viewport_rect.w) as f32 {
                idx += app.perspective.cols - col;
                break;
            }
            if yy > (view.viewport_rect.y + view.viewport_rect.h) as f32 || idx >= app.data.len() {
                break 'rows;
            }
            let byte = app.data[idx];
            let c = app
                .presentation
                .color_method
                .byte_color(byte, app.presentation.invert_color);
            drawfn(vertex_buffer, xx, yy, byte, c);
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
                //hex::hex(self, app, font, vertex_buffer);
                draw_view(
                    self,
                    app,
                    vertex_buffer,
                    |vertex_buffer, xx, yy, byte, c| {
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
                            xx + (self.col_w / 2) as f32 - 4.0,
                            yy,
                            d2.into(),
                            c,
                        );
                    },
                );
                rs.set_texture(Some(font.texture(app.layout.font_size.into())));
            }
            ViewKind::Ascii => {
                //ascii::ascii(self, app, font, vertex_buffer);
                draw_view(
                    self,
                    app,
                    vertex_buffer,
                    |vertex_buffer, xx, yy, byte, c| {
                        draw_glyph(
                            font,
                            app.layout.font_size.into(),
                            vertex_buffer,
                            xx,
                            yy,
                            u32::from(byte),
                            c,
                        );
                    },
                );
                rs.set_texture(Some(font.texture(app.layout.font_size.into())));
            }
            ViewKind::Block => {
                //block::block(self, app, window, vertex_buffer),
                draw_view(
                    self,
                    app,
                    vertex_buffer,
                    |vertex_buffer, xx, yy, _byte, c| {
                        draw_rect(
                            vertex_buffer,
                            xx,
                            yy,
                            self.col_w as f32,
                            self.row_h as f32,
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
