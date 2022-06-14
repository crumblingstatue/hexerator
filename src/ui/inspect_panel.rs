use std::{marker::PhantomData, str::FromStr};

use egui_sfml::egui::{self, Ui};
use sfml::system::Vector2i;

use crate::{app::App, damage_region::DamageRegion, InteractMode};

pub struct InspectPanel {
    input_thingies: [Box<dyn InputThingyTrait>; 9],
    /// True if an input thingy was changed by the user. Should update the others
    changed_one: bool,
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
                Box::new(InputThingy::<i8>::default()),
                Box::new(InputThingy::<u8>::default()),
                Box::new(InputThingy::<i16>::default()),
                Box::new(InputThingy::<u16>::default()),
                Box::new(InputThingy::<i32>::default()),
                Box::new(InputThingy::<u32>::default()),
                Box::new(InputThingy::<i64>::default()),
                Box::new(InputThingy::<u64>::default()),
                Box::new(InputThingy::<Ascii>::default()),
            ],
            changed_one: false,
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

trait NumBytesManip: std::fmt::Display + FromStr {
    type ToBytes: AsRef<[u8]>;
    fn label() -> &'static str;
    fn from_bytes(bytes: &[u8]) -> Self;
    fn to_bytes(&self) -> Self::ToBytes;
}

impl NumBytesManip for u8 {
    type ToBytes = [u8; 1];

    fn label() -> &'static str {
        "u8"
    }

    fn from_bytes(bytes: &[u8]) -> Self {
        bytes.first().cloned().unwrap_or_default()
    }

    fn to_bytes(&self) -> Self::ToBytes {
        [*self]
    }
}

impl NumBytesManip for i8 {
    type ToBytes = [u8; 1];

    fn label() -> &'static str {
        "i8"
    }

    fn from_bytes(bytes: &[u8]) -> Self {
        bytes.first().cloned().unwrap_or_default() as i8
    }

    fn to_bytes(&self) -> Self::ToBytes {
        [*self as u8]
    }
}

macro_rules! num_bytes_manip_impl {
    ($t:ty) => {
        impl NumBytesManip for $t {
            type ToBytes = [u8; <$t>::BITS as usize / 8];

            fn label() -> &'static str {
                stringify!($t)
            }

            fn from_bytes(bytes: &[u8]) -> Self {
                match bytes.get(..<$t>::BITS as usize / 8) {
                    Some(slice) => Self::from_le_bytes(slice.try_into().unwrap()),
                    None => Self::default(),
                }
            }

            fn to_bytes(&self) -> Self::ToBytes {
                self.to_le_bytes()
            }
        }
    };
}

num_bytes_manip_impl!(i16);
num_bytes_manip_impl!(u16);
num_bytes_manip_impl!(i32);
num_bytes_manip_impl!(u32);
num_bytes_manip_impl!(i64);
num_bytes_manip_impl!(u64);

impl<T: NumBytesManip> BytesManip for T {
    fn update_buf(buf: &mut String, data: &[u8], offset: usize) {
        if let Some(slice) = &data.get(offset..) {
            *buf = T::from_bytes(slice).to_string();
        }
    }

    fn label() -> &'static str {
        <Self as NumBytesManip>::label()
    }

    fn convert_and_write(buf: &str, data: &mut [u8], offset: usize) -> Option<DamageRegion> {
        match buf.parse::<Self>() {
            Ok(this) => {
                let bytes = this.to_bytes();
                let range = offset..offset + bytes.as_ref().len();
                match data.get_mut(range.clone()) {
                    Some(slice) => {
                        slice.copy_from_slice(bytes.as_ref());
                        Some(DamageRegion::Range(range))
                    }
                    None => None,
                }
            }
            Err(_) => None,
        }
    }
}

impl BytesManip for Ascii {
    fn update_buf(buf: &mut String, data: &[u8], offset: usize) {
        if let Some(slice) = &data.get(offset..) {
            let valid_ascii_end = find_valid_ascii_end(slice);
            *buf = String::from_utf8(data[offset..offset + valid_ascii_end].to_vec()).unwrap();
        }
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
    if offset != app.prev_frame_inspect_offset || app.just_reloaded || app.inspect_panel.changed_one
    {
        for thingy in &mut app.inspect_panel.input_thingies {
            thingy.update(&app.data[..], offset);
        }
    }
    app.inspect_panel.changed_one = false;
    let mut damages = Vec::new();
    for thingy in &mut app.inspect_panel.input_thingies {
        ui.label(thingy.label());
        if ui.text_edit_singleline(thingy.buf_mut()).lost_focus()
            && ui.input().key_pressed(egui::Key::Enter)
        {
            if let Some(range) = thingy.write_data(&mut app.data, offset) {
                app.inspect_panel.changed_one = true;
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
