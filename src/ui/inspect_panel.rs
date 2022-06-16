use std::{marker::PhantomData, str::FromStr};

use egui_sfml::egui::{self, Ui};
use sfml::{system::Vector2i, window::clipboard};

use crate::{
    app::{interact_mode::InteractMode, App},
    damage_region::DamageRegion,
};

pub struct InspectPanel {
    input_thingies: [Box<dyn InputThingyTrait>; 11],
    /// True if an input thingy was changed by the user. Should update the others
    changed_one: bool,
    big_endian: bool,
    hex: bool,
    /// If true, go to offset action is relative to the hard seek argument
    offset_relative: bool,
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
                Box::new(InputThingy::<f32>::default()),
                Box::new(InputThingy::<f64>::default()),
                Box::new(InputThingy::<Ascii>::default()),
            ],
            changed_one: false,
            big_endian: false,
            hex: false,
            offset_relative: false,
        }
    }
}

trait InputThingyTrait {
    fn update(&mut self, data: &[u8], offset: usize, be: bool, hex: bool);
    fn label(&self) -> &'static str;
    fn buf_mut(&mut self) -> &mut String;
    fn write_data(&self, data: &mut [u8], offset: usize, be: bool) -> Option<DamageRegion>;
}

impl<T: BytesManip> InputThingyTrait for InputThingy<T> {
    fn update(&mut self, data: &[u8], offset: usize, be: bool, hex: bool) {
        T::update_buf(&mut self.string, data, offset, be, hex);
    }
    fn label(&self) -> &'static str {
        T::label()
    }

    fn buf_mut(&mut self) -> &mut String {
        &mut self.string
    }

    fn write_data(&self, data: &mut [u8], offset: usize, be: bool) -> Option<DamageRegion> {
        T::convert_and_write(&self.string, data, offset, be)
    }
}

trait NumBytesManip: std::fmt::Display + FromStr {
    type ToBytes: AsRef<[u8]>;
    fn label() -> &'static str;
    fn from_le_bytes(bytes: &[u8]) -> Self;
    fn from_be_bytes(bytes: &[u8]) -> Self;
    fn to_le_bytes(&self) -> Self::ToBytes;
    fn to_be_bytes(&self) -> Self::ToBytes;
    fn to_hex_string(&self) -> String;
}

macro_rules! num_bytes_manip_impl {
    ($t:ty) => {
        impl NumBytesManip for $t {
            type ToBytes = [u8; <$t>::BITS as usize / 8];

            fn label() -> &'static str {
                stringify!($t)
            }

            fn from_le_bytes(bytes: &[u8]) -> Self {
                match bytes.get(..<$t>::BITS as usize / 8) {
                    Some(slice) => Self::from_le_bytes(slice.try_into().unwrap()),
                    None => Self::default(),
                }
            }

            fn from_be_bytes(bytes: &[u8]) -> Self {
                match bytes.get(..<$t>::BITS as usize / 8) {
                    Some(slice) => Self::from_be_bytes(slice.try_into().unwrap()),
                    None => Self::default(),
                }
            }

            fn to_le_bytes(&self) -> Self::ToBytes {
                <$t>::to_le_bytes(*self)
            }

            fn to_be_bytes(&self) -> Self::ToBytes {
                <$t>::to_be_bytes(*self)
            }

            fn to_hex_string(&self) -> String {
                format!("{:x}", self)
            }
        }
    };
}

num_bytes_manip_impl!(i8);
num_bytes_manip_impl!(u8);
num_bytes_manip_impl!(i16);
num_bytes_manip_impl!(u16);
num_bytes_manip_impl!(i32);
num_bytes_manip_impl!(u32);
num_bytes_manip_impl!(i64);
num_bytes_manip_impl!(u64);

impl NumBytesManip for f32 {
    type ToBytes = [u8; 32 / 8];

    fn label() -> &'static str {
        "f32"
    }

    fn from_le_bytes(bytes: &[u8]) -> Self {
        match bytes.get(..32 / 8) {
            Some(slice) => Self::from_le_bytes(slice.try_into().unwrap()),
            None => Self::default(),
        }
    }

    fn from_be_bytes(bytes: &[u8]) -> Self {
        match bytes.get(..32 / 8) {
            Some(slice) => Self::from_be_bytes(slice.try_into().unwrap()),
            None => Self::default(),
        }
    }

    fn to_le_bytes(&self) -> Self::ToBytes {
        f32::to_le_bytes(*self)
    }

    fn to_be_bytes(&self) -> Self::ToBytes {
        f32::to_be_bytes(*self)
    }

    fn to_hex_string(&self) -> String {
        "<no hex output>".into()
    }
}

impl NumBytesManip for f64 {
    type ToBytes = [u8; 64 / 8];

    fn label() -> &'static str {
        "f64"
    }

    fn from_le_bytes(bytes: &[u8]) -> Self {
        match bytes.get(..64 / 8) {
            Some(slice) => Self::from_le_bytes(slice.try_into().unwrap()),
            None => Self::default(),
        }
    }

    fn from_be_bytes(bytes: &[u8]) -> Self {
        match bytes.get(..64 / 8) {
            Some(slice) => Self::from_be_bytes(slice.try_into().unwrap()),
            None => Self::default(),
        }
    }

    fn to_le_bytes(&self) -> Self::ToBytes {
        f64::to_le_bytes(*self)
    }

    fn to_be_bytes(&self) -> Self::ToBytes {
        f64::to_le_bytes(*self)
    }

    fn to_hex_string(&self) -> String {
        "<no hex output>".into()
    }
}

impl<T: NumBytesManip> BytesManip for T {
    fn update_buf(buf: &mut String, data: &[u8], offset: usize, be: bool, hex: bool) {
        if let Some(slice) = &data.get(offset..) {
            let value = if be {
                T::from_be_bytes(slice)
            } else {
                T::from_le_bytes(slice)
            };
            if hex {
                *buf = value.to_hex_string();
            } else {
                *buf = value.to_string();
            }
        }
    }

    fn label() -> &'static str {
        <Self as NumBytesManip>::label()
    }

    fn convert_and_write(
        buf: &str,
        data: &mut [u8],
        offset: usize,
        be: bool,
    ) -> Option<DamageRegion> {
        match buf.parse::<Self>() {
            Ok(this) => {
                let bytes = if be {
                    this.to_be_bytes()
                } else {
                    this.to_le_bytes()
                };
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
    fn update_buf(buf: &mut String, data: &[u8], offset: usize, _be: bool, _hex: bool) {
        if let Some(slice) = &data.get(offset..) {
            let valid_ascii_end = find_valid_ascii_end(slice);
            *buf = String::from_utf8(data[offset..offset + valid_ascii_end].to_vec()).unwrap();
        }
    }

    fn label() -> &'static str {
        "ascii"
    }

    fn convert_and_write(
        buf: &str,
        data: &mut [u8],
        offset: usize,
        _be: bool,
    ) -> Option<DamageRegion> {
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
    fn update_buf(buf: &mut String, data: &[u8], offset: usize, be: bool, hex: bool);
    fn label() -> &'static str;
    fn convert_and_write(
        buf: &str,
        data: &mut [u8],
        offset: usize,
        be: bool,
    ) -> Option<DamageRegion>;
}

struct Ascii;

enum Action {
    GoToOffset(usize),
    AddDirty(DamageRegion),
    JumpForward(usize),
}

pub fn ui(ui: &mut Ui, app: &mut App, mouse_pos: Vector2i) {
    let offset = match app.interact_mode {
        InteractMode::View => {
            let off = app.pixel_pos_byte_offset(mouse_pos.x, mouse_pos.y);
            let mut add = 0;
            if app.ui.inspect_panel.offset_relative {
                add = app.args.hard_seek.unwrap_or(0) as usize;
            }
            ui.label(format!("offset: {} (0x{:x})", off + add, off + add));
            off
        }
        InteractMode::Edit => {
            let mut off = app.edit_state.cursor;
            if app.ui.inspect_panel.offset_relative {
                off += app.args.hard_seek.unwrap_or(0) as usize;
            }
            ui.label(format!("offset: {} ({:x}h)", off, off));
            app.edit_state.cursor
        }
    };
    ui.checkbox(&mut app.ui.inspect_panel.offset_relative, "Relative offset")
        .on_hover_text("Offset relative to --hard-seek");
    if app.data.is_empty() {
        return;
    }
    if offset != app.prev_frame_inspect_offset
        || app.just_reloaded
        || app.ui.inspect_panel.changed_one
    {
        for thingy in &mut app.ui.inspect_panel.input_thingies {
            thingy.update(
                &app.data[..],
                offset,
                app.ui.inspect_panel.big_endian,
                app.ui.inspect_panel.hex,
            );
        }
    }
    app.ui.inspect_panel.changed_one = false;
    let mut actions = Vec::new();
    for thingy in &mut app.ui.inspect_panel.input_thingies {
        ui.horizontal(|ui| {
            ui.label(thingy.label());
            if ui.button("📋").on_hover_text("copy to clipboard").clicked() {
                clipboard::set_string(&*thingy.buf_mut());
            }
            if ui.button("⬇").on_hover_text("go to offset").clicked() {
                let offset = if app.ui.inspect_panel.hex {
                    usize::from_str_radix(thingy.buf_mut(), 16).unwrap()
                } else {
                    thingy.buf_mut().parse().unwrap()
                };
                actions.push(Action::GoToOffset(offset));
            }
            if ui.button("➡").on_hover_text("jump forward").clicked() {
                let offset = if app.ui.inspect_panel.hex {
                    usize::from_str_radix(thingy.buf_mut(), 16).unwrap()
                } else {
                    thingy.buf_mut().parse().unwrap()
                };
                actions.push(Action::JumpForward(offset));
            }
        });
        if ui.text_edit_singleline(thingy.buf_mut()).lost_focus()
            && ui.input().key_pressed(egui::Key::Enter)
        {
            if let Some(range) =
                thingy.write_data(&mut app.data, offset, app.ui.inspect_panel.big_endian)
            {
                app.ui.inspect_panel.changed_one = true;
                actions.push(Action::AddDirty(range));
            }
        }
    }
    ui.horizontal(|ui| {
        if ui
            .checkbox(&mut app.ui.inspect_panel.big_endian, "Big endian")
            .clicked()
        {
            // Changing this should refresh everything
            app.ui.inspect_panel.changed_one = true;
        }
        if ui.checkbox(&mut app.ui.inspect_panel.hex, "Hex").clicked() {
            // Changing this should refresh everything
            app.ui.inspect_panel.changed_one = true;
        }
    });

    for action in actions {
        match action {
            Action::GoToOffset(offset) => {
                if app.ui.inspect_panel.offset_relative {
                    app.edit_state
                        .set_cursor(offset - app.args.hard_seek.unwrap_or(0) as usize);
                } else {
                    app.edit_state.set_cursor(offset);
                }
                app.center_view_on_offset(app.edit_state.cursor);
                app.flash_cursor();
            }
            Action::AddDirty(damage) => app.widen_dirty_region(damage),
            Action::JumpForward(amount) => {
                app.edit_state.set_cursor(app.edit_state.cursor + amount);
                app.center_view_on_offset(app.edit_state.cursor);
                app.flash_cursor();
            }
        }
    }
    app.prev_frame_inspect_offset = offset;
}

fn find_valid_ascii_end(data: &[u8]) -> usize {
    data.iter()
        .position(|&b| b == 0 || b > 127)
        .unwrap_or(data.len())
}
