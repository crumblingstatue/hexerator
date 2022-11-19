use {
    super::View,
    crate::{
        app::{presentation::Presentation, App},
        color::RgbColor,
        dec_conv,
        gui::Gui,
        hex_conv,
        meta::{region::Region, PerspectiveMap, RegionMap, ViewKey},
        view::ViewKind,
    },
    egui_sfml::sfml::{
        graphics::{
            Color, Font, PrimitiveType, RenderStates, RenderTarget, RenderWindow, Text,
            Transformable, Vertex,
        },
        system::Vector2,
    },
    either::Either,
    glu_sys::GLint,
    slotmap::Key,
};

struct DrawArgs<'vert, 'data> {
    vertices: &'vert mut Vec<Vertex>,
    x: f32,
    y: f32,
    data: &'data [u8],
    idx: usize,
    color: RgbColor,
    highlight: bool,
}

fn draw_view(
    view: &View,
    app_perspectives: &PerspectiveMap,
    app_regions: &RegionMap,
    app_data: &[u8],
    app_selection: Option<Region>,
    app_ui: &Gui,
    vertex_buffer: &mut Vec<Vertex>,
    mut drawfn: impl FnMut(DrawArgs),
) {
    // Protect against infinite loop lock up when scrolling horizontally out of view
    if view.scroll_offset.pix_xoff <= -view.viewport_rect.w || view.perspective.is_null() {
        return;
    }
    let perspective = &app_perspectives[view.perspective];
    let region = &app_regions[perspective.region].region;
    let mut idx = region.begin;
    let start_row: usize = view.scroll_offset.row;
    idx += start_row * (perspective.cols * usize::from(view.bytes_per_block));
    #[expect(
        clippy::cast_sign_loss,
        reason = "rows() returning negative is a bug, should be positive."
    )]
    let orig = start_row..=start_row + view.rows() as usize;
    let (row_range, pix_yoff) = if perspective.flip_row_order {
        (Either::Left(orig.rev()), -view.scroll_offset.pix_yoff)
    } else {
        (Either::Right(orig), view.scroll_offset.pix_yoff)
    };
    'rows: for row in row_range {
        let y = row * usize::from(view.row_h);
        let viewport_y = (i64::from(view.viewport_rect.y) + y as i64)
            - ((view.scroll_offset.row as i64 * i64::from(view.row_h)) + i64::from(pix_yoff));
        let start_col = view.scroll_offset.col;
        if start_col >= perspective.cols {
            break;
        }
        idx += start_col * usize::from(view.bytes_per_block);
        for col in start_col..perspective.cols {
            let x = col * usize::from(view.col_w);
            let viewport_x = (i64::from(view.viewport_rect.x) + x as i64)
                - ((view.scroll_offset.col as i64 * i64::from(view.col_w))
                    + i64::from(view.scroll_offset.pix_xoff));
            if viewport_x > i64::from(view.viewport_rect.x + view.viewport_rect.w) {
                idx += (perspective.cols - col) * usize::from(view.bytes_per_block);
                break;
            }
            if idx > region.end {
                break 'rows;
            }
            if viewport_y > i64::from(view.viewport_rect.y + view.viewport_rect.h)
                && !perspective.flip_row_order
            {
                break 'rows;
            }
            match app_data.get(idx..idx + view.bytes_per_block as usize) {
                Some(data) => {
                    let c = view
                        .presentation
                        .color_method
                        .byte_color(data[0], view.presentation.invert_color);
                    #[expect(
                        clippy::cast_precision_loss,
                        reason = "At this point, the viewport coordinates should be small enough to fit in viewport"
                    )]
                    drawfn(DrawArgs {
                        vertices: vertex_buffer,
                        x: viewport_x as f32,
                        y: viewport_y as f32,
                        data,
                        idx,
                        color: c,
                        highlight: should_highlight(app_selection, idx, app_ui),
                    });
                    /*if gamedebug_core::enabled() {
                        #[expect(
                            clippy::cast_precision_loss,
                            reason = "At this point, the viewport coordinates should be small enough to fit in viewport"
                        )]
                        draw_rect_outline(
                            vertex_buffer,
                            viewport_x as f32,
                            viewport_y as f32,
                            view.col_w.into(),
                            view.row_h.into(),
                            Color::RED,
                            -1.0,
                        );
                    }*/
                    idx += usize::from(view.bytes_per_block);
                }
                None => {
                    if !perspective.flip_row_order {
                        break 'rows;
                    }
                }
            }
        }
    }
}

fn draw_text_cursor(
    x: f32,
    y: f32,
    vertices: &mut Vec<Vertex>,
    active: bool,
    flash_timer: Option<u32>,
    presentation: &Presentation,
    font_size: u16,
) {
    let color = cursor_color(active, flash_timer, presentation);
    draw_rect_outline(
        vertices,
        x,
        y,
        f32::from(font_size / 2),
        f32::from(font_size),
        color,
        -2.0,
    );
}

fn draw_block_cursor(
    x: f32,
    y: f32,
    vertices: &mut Vec<Vertex>,
    active: bool,
    flash_timer: Option<u32>,
    presentation: &Presentation,
    view: &View,
) {
    let color = cursor_color(active, flash_timer, presentation);
    draw_rect(
        vertices,
        x,
        y,
        f32::from(view.col_w),
        f32::from(view.row_h),
        color,
    );
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "Deliberate color modulation based on timer value."
)]
fn cursor_color(active: bool, flash_timer: Option<u32>, presentation: &Presentation) -> Color {
    if active {
        match flash_timer {
            Some(timer) => Color::rgb(timer as u8, timer as u8, timer as u8),
            None => presentation.cursor_active_color.into(),
        }
    } else {
        match flash_timer {
            Some(timer) => Color::rgb(timer as u8, timer as u8, timer as u8),
            None => presentation.cursor_color.into(),
        }
    }
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
    let texture_rect = glyph.texture_rect();
    let baseline = font_size as f32;
    let offset = baseline + bounds.top;
    x += bounds.left;
    y += offset;
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
        key: ViewKey,
        app: &App,
        gui: &Gui,
        window: &mut RenderWindow,
        vertex_buffer: &mut Vec<Vertex>,
        font: &Font,
    ) {
        vertex_buffer.clear();
        let mut rs = RenderStates::default();
        let this = &app.meta_state.meta.views[key];
        match &this.view.kind {
            ViewKind::Hex(hex) => {
                draw_view(
                    &this.view,
                    &app.meta_state.meta.low.perspectives,
                    &app.meta_state.meta.low.regions,
                    &app.data,
                    app.hex_ui.selection(),
                    gui,
                    vertex_buffer,
                    |DrawArgs {
                         vertices,
                         x,
                         y,
                         data,
                         idx,
                         color: c,
                         highlight,
                     }| {
                        if highlight {
                            draw_rect(
                                vertices,
                                x,
                                y,
                                f32::from(this.view.col_w),
                                f32::from(this.view.row_h),
                                this.view.presentation.sel_color.into(),
                            )
                        }
                        let mut gx = x;
                        for (i, mut d) in hex_conv::byte_to_hex_digits(data[0])
                            .into_iter()
                            .enumerate()
                        {
                            if idx == app.edit_state.cursor && hex.edit_buf.dirty {
                                d = hex.edit_buf.buf[i];
                            }
                            draw_glyph(
                                font,
                                hex.font_size.into(),
                                vertices,
                                gx,
                                y,
                                d.into(),
                                c.into(),
                            );
                            gx += f32::from(hex.font_size - 4);
                        }
                        let extra_x = hex.edit_buf.cursor * (hex.font_size - 4);
                        if !app.preferences.hide_cursor && idx == app.edit_state.cursor {
                            draw_text_cursor(
                                x + f32::from(extra_x),
                                y,
                                vertices,
                                app.hex_ui.focused_view == Some(key),
                                app.hex_ui.cursor_flash_timer(),
                                &this.view.presentation,
                                hex.font_size,
                            );
                        }
                    },
                );
                rs.set_texture(Some(font.texture(hex.font_size.into())));
            }
            ViewKind::Dec(dec) => {
                draw_view(
                    &this.view,
                    &app.meta_state.meta.low.perspectives,
                    &app.meta_state.meta.low.regions,
                    &app.data,
                    app.hex_ui.selection(),
                    gui,
                    vertex_buffer,
                    |DrawArgs {
                         vertices,
                         x,
                         y,
                         data,
                         idx,
                         color: c,
                         highlight,
                     }| {
                        if highlight {
                            draw_rect(
                                vertices,
                                x,
                                y,
                                f32::from(this.view.col_w),
                                f32::from(this.view.row_h),
                                this.view.presentation.sel_color.into(),
                            )
                        }
                        let mut gx = x;
                        for (i, mut d) in dec_conv::byte_to_dec_digits(data[0])
                            .into_iter()
                            .enumerate()
                        {
                            if idx == app.edit_state.cursor && dec.edit_buf.dirty {
                                d = dec.edit_buf.buf[i];
                            }
                            draw_glyph(
                                font,
                                dec.font_size.into(),
                                vertices,
                                gx,
                                y,
                                d.into(),
                                c.into(),
                            );
                            gx += f32::from(dec.font_size - 4);
                        }
                        let extra_x = dec.edit_buf.cursor * (dec.font_size - 4);
                        if !app.preferences.hide_cursor && idx == app.edit_state.cursor {
                            draw_text_cursor(
                                x + f32::from(extra_x),
                                y,
                                vertices,
                                app.hex_ui.focused_view == Some(key),
                                app.hex_ui.cursor_flash_timer(),
                                &this.view.presentation,
                                dec.font_size,
                            );
                        }
                    },
                );
                rs.set_texture(Some(font.texture(dec.font_size.into())));
            }
            ViewKind::Text(text) => {
                draw_view(
                    &this.view,
                    &app.meta_state.meta.low.perspectives,
                    &app.meta_state.meta.low.regions,
                    &app.data,
                    app.hex_ui.selection(),
                    gui,
                    vertex_buffer,
                    |DrawArgs {
                         vertices,
                         x,
                         y,
                         data,
                         idx,
                         color: c,
                         highlight,
                     }| {
                        if highlight {
                            draw_rect(
                                vertices,
                                x,
                                y,
                                f32::from(this.view.col_w),
                                f32::from(this.view.row_h),
                                this.view.presentation.sel_color.into(),
                            )
                        }
                        let raw_data = match text.text_kind {
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
                        draw_glyph(font, text.font_size.into(), vertices, x, y, glyph, c.into());
                        if !app.preferences.hide_cursor && idx == app.edit_state.cursor {
                            draw_text_cursor(
                                x,
                                y,
                                vertices,
                                app.hex_ui.focused_view == Some(key),
                                app.hex_ui.cursor_flash_timer(),
                                &this.view.presentation,
                                text.font_size,
                            );
                        }
                    },
                );
                rs.set_texture(Some(font.texture(text.font_size.into())));
            }
            ViewKind::Block => {
                draw_view(
                    &this.view,
                    &app.meta_state.meta.low.perspectives,
                    &app.meta_state.meta.low.regions,
                    &app.data,
                    app.hex_ui.selection(),
                    gui,
                    vertex_buffer,
                    |DrawArgs {
                         vertices,
                         x,
                         y,
                         data: _,
                         idx,
                         color: mut c,
                         highlight,
                     }| {
                        if highlight {
                            c = c.invert();
                        }
                        draw_rect(
                            vertices,
                            x,
                            y,
                            f32::from(this.view.col_w),
                            f32::from(this.view.row_h),
                            c.into(),
                        );
                        if !app.preferences.hide_cursor && idx == app.edit_state.cursor {
                            draw_block_cursor(
                                x,
                                y,
                                vertices,
                                app.hex_ui.focused_view == Some(key),
                                app.hex_ui.cursor_flash_timer(),
                                &this.view.presentation,
                                &this.view,
                            );
                        }
                    },
                );
            }
        }
        draw_rect_outline(
            vertex_buffer,
            f32::from(this.view.viewport_rect.x - 2),
            f32::from(this.view.viewport_rect.y - 2),
            f32::from(this.view.viewport_rect.w + 3),
            f32::from(this.view.viewport_rect.h + 3),
            if Some(key) == app.hex_ui.focused_view {
                Color::rgb(255, 255, 150)
            } else {
                Color::rgb(120, 120, 150)
            },
            -1.0,
        );
        if app.hex_ui.scissor_views {
            unsafe {
                glu_sys::glEnable(glu_sys::GL_SCISSOR_TEST);
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "Huge window sizes (>32000) are not supported."
                )]
                let vh = window.size().y as i16;
                let (x, y, w, h) = rect_to_gl_viewport(
                    this.view.viewport_rect.x - 2,
                    this.view.viewport_rect.y - 2,
                    this.view.viewport_rect.w + 3,
                    this.view.viewport_rect.h + 3,
                    vh,
                );
                glu_sys::glScissor(x, y, w, h);
            }
        }
        let mut overlay_text = None;
        if app.hex_ui.show_alt_overlay {
            let mut text = Text::new(&this.name, font, 16);
            text.set_position((
                f32::from(this.view.viewport_rect.x),
                f32::from(this.view.viewport_rect.y),
            ));
            let text_bounds = text.global_bounds();
            draw_rect(
                vertex_buffer,
                text_bounds.left,
                text_bounds.top,
                text_bounds.width,
                text_bounds.height,
                Color::rgba(32, 32, 32, 200),
            );
            overlay_text = Some(text);
        }
        window.draw_primitives(vertex_buffer, PrimitiveType::QUADS, &rs);
        if app.hex_ui.scissor_views {
            unsafe {
                glu_sys::glDisable(glu_sys::GL_SCISSOR_TEST);
            }
        }
        if let Some(text) = overlay_text {
            window.draw(&text);
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

fn should_highlight(app_selection: Option<Region>, idx: usize, gui: &Gui) -> bool {
    selected(app_selection, idx) || gui.highlight_set.contains(&idx)
}

fn selected(app_selection: Option<Region>, idx: usize) -> bool {
    match app_selection {
        Some(sel) => (sel.begin..=sel.end).contains(&idx),
        None => false,
    }
}
