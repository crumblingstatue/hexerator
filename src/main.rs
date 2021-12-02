mod hex_conv;

use sfml::{
    graphics::{Color, Font, PrimitiveType, RenderStates, RenderTarget, RenderWindow, Vertex},
    system::Vector2,
    window::{ContextSettings, Event, Style},
};

fn main() {
    let path = std::env::args_os()
        .nth(1)
        .expect("Need file path as argument");
    let data = std::fs::read(path).unwrap();
    let mut w = RenderWindow::new(
        (1920, 1080),
        "hello",
        Style::NONE,
        &ContextSettings::default(),
    );
    w.set_vertical_sync_enabled(true);
    w.set_position(Vector2::new(0, 0));
    let f = Font::from_file("DejaVuSansMono.ttf").unwrap();
    let mut vertices = Vec::new();

    while w.is_open() {
        while let Some(event) = w.poll_event() {
            match event {
                Event::Closed => w.close(),
                _ => {}
            }
        }
        w.clear(Color::BLACK);
        let mut rs = RenderStates::default();
        vertices.clear();
        let mut idx = 0;
        for y in 0..67 {
            for x in 0..73 {
                let byte = data[idx];
                let [g1, g2] = hex_conv::byte_to_hex_digits(byte);
                draw_glyph(
                    &f,
                    &mut vertices,
                    x as f32 * 26.0,
                    y as f32 * 16.0,
                    g1 as u32,
                    Color::WHITE,
                );
                draw_glyph(
                    &f,
                    &mut vertices,
                    x as f32 * 26.0 + 11.0,
                    y as f32 * 16.0,
                    g2 as u32,
                    Color::WHITE,
                );
                idx += 1;
            }
        }
        rs.set_texture(Some(f.texture(10)));
        w.draw_primitives(&vertices, PrimitiveType::QUADS, &rs);
        rs.set_texture(None);
        w.display();
    }
}

fn draw_glyph(font: &Font, vertices: &mut Vec<Vertex>, x: f32, y: f32, glyph: u32, color: Color) {
    let g = font.glyph(glyph, 10, false, 0.0);
    let r = g.texture_rect();
    vertices.push(Vertex {
        position: Vector2::new(x, y),
        color,
        tex_coords: Vector2::new(r.left as f32, r.top as f32),
    });
    vertices.push(Vertex {
        position: Vector2::new(x, y + 10.0),
        color,
        tex_coords: Vector2::new(r.left as f32, (r.top + r.height) as f32),
    });
    vertices.push(Vertex {
        position: Vector2::new(x + 10.0, y + 10.0),
        color,
        tex_coords: Vector2::new((r.left + r.width) as f32, (r.top + r.height) as f32),
    });
    vertices.push(Vertex {
        position: Vector2::new(x + 10.0, y),
        color,
        tex_coords: Vector2::new((r.left + r.width) as f32, r.top as f32),
    });
}
