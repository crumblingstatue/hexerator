mod ascii;
mod block;
mod hex;
pub use ascii::ascii;
pub use block::block;

use glu_sys::GLint;
pub use hex::hex;
use sfml::{
    graphics::{Color, Font, PrimitiveType, RenderStates, RenderTarget, RenderWindow, Vertex},
    system::Vector2,
};

use crate::app::{presentation::Presentation, App};

/// A rendering of data in a view.
///
/// There can be different lenses into the same view, like a hex lens, ascii lens, block lens...
/// They all sync on the same view offset, but each lens can show different amounts of data
/// depending on block size of its items, and its relative size in the viewport.
///
/// The positions all count from the window, so they're always the position relative to the window.
#[derive(Debug)]
pub struct Lens {
    /// The rectangle to occupy in the viewport
    pub viewport_rect: ViewportRect,
    /// The kind of lens (hex, ascii, block, etc)
    pub kind: LensKind,
    /// Width of a column
    pub col_w: u8,
    /// Height of a row
    pub row_h: u8,
}

#[derive(Debug)]
pub struct ViewportRect {
    pub x: i16,
    pub y: i16,
    pub w: i16,
    pub h: i16,
}

/// The kind of lens (hex, ascii, block, etc)
#[derive(Debug)]
pub enum LensKind {
    Hex,
    Ascii,
    Block,
}

impl Lens {
    pub fn draw(
        &self,
        app: &mut App,
        window: &mut RenderWindow,
        vertex_buffer: &mut Vec<Vertex>,
        font: &Font,
    ) {
        //app.scissor_lenses = false;
        vertex_buffer.clear();
        let mut rs = RenderStates::default();
        match self.kind {
            LensKind::Hex => {
                hex::hex(self, app, font, vertex_buffer);
                rs.set_texture(Some(font.texture(app.layout.font_size.into())));
            }
            LensKind::Ascii => {
                ascii::ascii(self, app, font, vertex_buffer);
                rs.set_texture(Some(font.texture(app.layout.font_size.into())));
            }
            LensKind::Block => block::block(self, app, window, vertex_buffer),
        }
        draw_rect_outline(
            vertex_buffer,
            self.viewport_rect.x.into(),
            self.viewport_rect.y.into(),
            self.viewport_rect.w.into(),
            self.viewport_rect.h.into(),
            Color::WHITE,
            -1.0,
        );
        if app.scissor_lenses {
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
        if app.scissor_lenses {
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
