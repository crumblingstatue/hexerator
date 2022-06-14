use std::marker::PhantomData;

use egui_sfml::egui::{self, Ui};
use sfml::system::Vector2i;

use crate::{app::App, InteractMode};

use super::DamageRegion;

pub struct InspectPanel {
    input_thingies: [Box<dyn InputThingyTrait>; 3],
}

impl std::fmt::Debug for InspectPanel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InspectPanel").finish()
    }
}

impl Default for InspectPanel {
    fn default() -> Self {
        Self {
            input_thingies: [
                Box::new(InputThingy::<u8>::default()),
                Box::new(InputThingy::<u16>::default()),
                Box::new(InputThingy::<Ascii>::default()),
            ],
        }
    }
}

trait InputThingyTrait {
    fn update(&mut self, data: &[u8], offset: usize);
    fn label(&self) -> &'static str;
    fn buf_mut(&mut self) -> &mut String;
    fn write_data(&self, data: &mut [u8], offset: usize) -> Option<DamageRegion>;
}

impl<T: BytesManip> InputThingyTrait for InputThingy<T> {
    fn update(&mut self, data: &[u8], offset: usize) {
        T::update_buf(&mut self.string, data, offset);
    }
    fn label(&self) -> &'static str {
        T::label()
    }

    fn buf_mut(&mut self) -> &mut String {
        &mut self.string
    }

    fn write_data(&self, data: &mut [u8], offset: usize) -> Option<DamageRegion> {
        T::convert_and_write(&self.string, data, offset)
    }
}

impl BytesManip for u8 {
    fn update_buf(buf: &mut String, data: &[u8], offset: usize) {
        *buf = data[offset].to_string()
    }

    fn label() -> &'static str {
        "u8"
    }

    fn convert_and_write(buf: &str, data: &mut [u8], offset: usize) -> Option<DamageRegion> {
        match buf.parse() {
            Ok(num) => {
                data[offset] = num;
                Some(DamageRegion::Single(offset))
            }
            Err(_) => None,
        }
    }
}
impl BytesManip for u16 {
    fn update_buf(buf: &mut String, data: &[u8], offset: usize) {
        let u16 = u16::from_le_bytes(data[offset..offset + 2].try_into().unwrap());
        *buf = u16.to_string();
    }

    fn label() -> &'static str {
        "u16"
    }

    fn convert_and_write(buf: &str, data: &mut [u8], offset: usize) -> Option<DamageRegion> {
        match buf.parse::<u16>() {
            Ok(num) => {
                let range = offset..offset + 2;
                data[range.clone()].copy_from_slice(&num.to_le_bytes());
                Some(DamageRegion::Range(range))
            }
            Err(_) => None,
        }
    }
}
impl BytesManip for Ascii {
    fn update_buf(buf: &mut String, data: &[u8], offset: usize) {
        let valid_ascii_end = find_valid_ascii_end(&data[offset..]);
        *buf = String::from_utf8(data[offset..offset + valid_ascii_end].to_vec()).unwrap();
    }

    fn label() -> &'static str {
        "ascii"
    }

    fn convert_and_write(buf: &str, data: &mut [u8], offset: usize) -> Option<DamageRegion> {
        let len = buf.len();
        let range = offset..offset + len;
        data[range.clone()].copy_from_slice(buf.as_bytes());
        Some(DamageRegion::Range(range))
    }
}

struct InputThingy<T> {
    string: String,
    _phantom: PhantomData<T>,
}

impl<T> Default for InputThingy<T> {
    fn default() -> Self {
        Self {
            string: Default::default(),
            _phantom: Default::default(),
        }
    }
}

trait BytesManip {
    fn update_buf(buf: &mut String, data: &[u8], offset: usize);
    fn label() -> &'static str;
    fn convert_and_write(buf: &str, data: &mut [u8], offset: usize) -> Option<DamageRegion>;
}

struct Ascii;

pub fn inspect_panel_ui(ui: &mut Ui, app: &mut App, mouse_pos: Vector2i) {
    let offset = match app.interact_mode {
        InteractMode::View => {
            let off = app.pixel_pos_byte_offset(mouse_pos.x, mouse_pos.y);
            ui.label(format!("Pointer at {} (0x{:x})", off, off));
            off
        }
        InteractMode::Edit => {
            ui.label(format!("Cursor at {} ({:x}h)", app.cursor, app.cursor));
            app.cursor
        }
    };
    if app.data.is_empty() {
        return;
    }
    if offset != app.prev_frame_inspect_offset {
        for thingy in &mut app.inspect_panel.input_thingies {
            thingy.update(&app.data[..], offset);
        }
    }
    let mut damages = Vec::new();
    for thingy in &mut app.inspect_panel.input_thingies {
        ui.label(thingy.label());
        if ui.text_edit_singleline(thingy.buf_mut()).lost_focus()
            && ui.input().key_pressed(egui::Key::Enter)
        {
            if let Some(range) = thingy.write_data(&mut app.data, offset) {
                damages.push(range);
            }
        }
    }
    for damage in damages {
        app.widen_dirty_region(damage);
    }
    app.prev_frame_inspect_offset = offset;
}

fn find_valid_ascii_end(data: &[u8]) -> usize {
    data.iter()
        .position(|&b| b == 0 || b > 127)
        .unwrap_or(data.len())
}
