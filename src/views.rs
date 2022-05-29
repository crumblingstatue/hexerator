mod ascii;
mod block;
mod hex;
pub use ascii::ascii;
pub use block::block;
use egui_sfml::egui::color::rgb_from_hsv;
pub use hex::hex;
use sfml::{
    graphics::{Color, Font, Rect, RectangleShape, RenderTarget, RenderWindow, Shape, Vertex},
    system::Vector2,
};

fn byte_color(byte: u8, mono: bool) -> Color {
    if mono {
        Color::WHITE
    } else if byte == 0 {
        Color::rgb(100, 100, 100)
    } else if byte == 255 {
        Color::WHITE
    } else {
        let [r, g, b] = rgb_from_hsv((byte as f32 / 288.0, 1.0, 1.0));
        Color::rgb((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
    }
}

fn draw_cursor(x: f32, y: f32, window: &mut RenderWindow, active: bool) {
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
    window.draw(&rs);
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
        tex_coords: Vector2::new(texture_rect.left as f32, texture_rect.top as f32),
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
