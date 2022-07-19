mod ascii;
mod block;
mod hex;
mod generic;
pub use ascii::ascii;
pub use block::block;

use glu_sys::GLint;
pub use hex::hex;
use sfml::{
    graphics::{Color, Font, PrimitiveType, RenderStates, RenderTarget, RenderWindow, Vertex},
    system::Vector2,
};

use crate::{app::{presentation::Presentation, App}, hex_conv};

/// A rectangular view in the viewport looking through a perspective at the data with a flavor
/// of rendering/interaction (hex/ascii/block/etc.)
///
/// There can be different views through the same perspective.
/// By default they sync their offsets, but each view can show different amounts of data
/// depending on block size of its items, and its relative size in the viewport.
#[derive(Debug)]
pub struct View {
    /// The rectangle to occupy in the viewport
    pub viewport_rect: ViewportRect,
    /// The kind of view (hex, ascii, block, etc)
    pub kind: ViewKind,
    /// Width of a column
    pub col_w: u8,
    /// Height of a row
    pub row_h: u8,
    /// The scrolling offset
    pub scroll_offset: ScrollOffset,
    /// The amount scrolled for a single scroll operation, in pixels
    pub scroll_speed: i16,
}

#[derive(Debug)]
pub struct ScrollOffset {
    /// What column we are at
    pub col_x: usize,
    /// Additional pixel x offset
    pub pix_x: i16,
    /// What row we are at
    pub row_y: usize,
    /// Additional pixel y offset
    pub pix_y: i16,
}

#[derive(Debug)]
pub struct ViewportRect {
    pub x: i16,
    pub y: i16,
    pub w: i16,
    pub h: i16,
}

/// The kind of view (hex, ascii, block, etc)
#[derive(Debug)]
pub enum ViewKind {
    Hex,
    Ascii,
    Block,
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
                generic::generic(self, app, window, vertex_buffer, |vertex_buffer, xx, yy, byte, c| {
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
                });
                rs.set_texture(Some(font.texture(app.layout.font_size.into())));
            }
            ViewKind::Ascii => {
                //ascii::ascii(self, app, font, vertex_buffer);
                generic::generic(self, app, window, vertex_buffer, |vertex_buffer, xx, yy, byte, c| {
                    draw_glyph(
                        font,
                        app.layout.font_size.into(),
                        vertex_buffer,
                        xx,
                        yy,
                        u32::from(byte),
                        c,
                    );
                });
                rs.set_texture(Some(font.texture(app.layout.font_size.into())));
            }
            ViewKind::Block => {
                //block::block(self, app, window, vertex_buffer),
                generic::generic(self, app, window, vertex_buffer, |vertex_buffer, xx, yy, byte, c| {
                    draw_rect(
                        vertex_buffer,
                        xx,
                        yy,
                        self.col_w as f32,
                        self.row_h as f32,
                        c,
                    );
                });
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
        if app.scissor_views {
            unsafe {
                glu_sys::glDisable(glu_sys::GL_SCISSOR_TEST);
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
