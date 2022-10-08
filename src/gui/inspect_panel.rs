use {
    super::message_dialog::{Icon, MessageDialog},
    crate::{
        app::{interact_mode::InteractMode, App},
        damage_region::DamageRegion,
        shell::msg_if_fail,
        view::ViewportVec,
    },
    anyhow::bail,
    egui::{self, Ui},
    slotmap::Key,
    std::{array::TryFromSliceError, marker::PhantomData},
    thiserror::Error,
};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Format {
    Decimal,
    Hex,
    Bin,
}

impl Format {
    fn label(&self) -> &'static str {
        match self {
            Format::Decimal => "Decimal",
            Format::Hex => "Hex",
            Format::Bin => "Binary",
        }
    }
}

pub struct InspectPanel {
    input_thingies: [Box<dyn InputThingyTrait>; 11],
    /// True if an input thingy was changed by the user. Should update the others
    changed_one: bool,
    big_endian: bool,
    format: Format,
    /// If true, go to offset action is relative to the hard seek argument
    offset_relative: bool,
    /// The value of the cursor on the previous frame. Used to determine when the cursor changes
    pub prev_frame_inspect_offset: usize,
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
                Box::<InputThingy<i8>>::default(),
                Box::<InputThingy<u8>>::default(),
                Box::<InputThingy<i16>>::default(),
                Box::<InputThingy<u16>>::default(),
                Box::<InputThingy<i32>>::default(),
                Box::<InputThingy<u32>>::default(),
                Box::<InputThingy<i64>>::default(),
                Box::<InputThingy<u64>>::default(),
                Box::<InputThingy<f32>>::default(),
                Box::<InputThingy<f64>>::default(),
                Box::<InputThingy<Ascii>>::default(),
            ],
            changed_one: false,
            big_endian: false,
            format: Format::Decimal,
            offset_relative: false,
            prev_frame_inspect_offset: 0,
        }
    }
}

trait InputThingyTrait {
    fn update(&mut self, data: &[u8], offset: usize, be: bool, format: Format);
    fn label(&self) -> &'static str;
    fn buf_mut(&mut self) -> &mut String;
    fn write_data(
        &self,
        data: &mut [u8],
        offset: usize,
        be: bool,
        format: Format,
        msg: &mut MessageDialog,
    ) -> Option<DamageRegion>;
}

impl<T: BytesManip> InputThingyTrait for InputThingy<T> {
    fn update(&mut self, data: &[u8], offset: usize, be: bool, format: Format) {
        T::update_buf(&mut self.string, data, offset, be, format);
    }
    fn label(&self) -> &'static str {
        T::label()
    }

    fn buf_mut(&mut self) -> &mut String {
        &mut self.string
    }

    fn write_data(
        &self,
        data: &mut [u8],
        offset: usize,
        be: bool,
        format: Format,
        msg: &mut MessageDialog,
    ) -> Option<DamageRegion> {
        T::convert_and_write(&self.string, data, offset, be, format, msg)
    }
}

#[derive(Error, Debug)]
enum FromBytesError {
    #[error("Error converting from slice")]
    TryFromSlice(#[from] TryFromSliceError),
    #[error("Error indexing slice")]
    SliceIndexError,
}

trait NumBytesManip: std::fmt::Display + Sized {
    type ToBytes: AsRef<[u8]>;
    fn label() -> &'static str;
    fn from_le_bytes(bytes: &[u8]) -> Result<Self, FromBytesError>;
    fn from_be_bytes(bytes: &[u8]) -> Result<Self, FromBytesError>;
    fn to_le_bytes(&self) -> Self::ToBytes;
    fn to_be_bytes(&self) -> Self::ToBytes;
    fn to_hex_string(&self) -> String;
    fn to_bin_string(&self) -> String;
    fn from_str(input: &str, format: Format) -> Result<Self, anyhow::Error>;
}

macro_rules! num_bytes_manip_impl {
    ($t:ty) => {
        impl NumBytesManip for $t {
            type ToBytes = [u8; <$t>::BITS as usize / 8];

            fn label() -> &'static str {
                stringify!($t)
            }

            fn from_le_bytes(bytes: &[u8]) -> Result<Self, FromBytesError> {
                match bytes.get(..<$t>::BITS as usize / 8) {
                    Some(slice) => Ok(Self::from_le_bytes(slice.try_into()?)),
                    None => Err(FromBytesError::SliceIndexError),
                }
            }

            fn from_be_bytes(bytes: &[u8]) -> Result<Self, FromBytesError> {
                match bytes.get(..<$t>::BITS as usize / 8) {
                    Some(slice) => Ok(Self::from_be_bytes(slice.try_into()?)),
                    None => Err(FromBytesError::SliceIndexError),
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

            fn to_bin_string(&self) -> String {
                format!("{:0w$b}", self, w = <$t>::BITS as usize)
            }

            fn from_str(input: &str, format: Format) -> Result<Self, anyhow::Error> {
                let this = match format {
                    Format::Decimal => input.parse()?,
                    Format::Hex => Self::from_str_radix(input, 16)?,
                    Format::Bin => Self::from_str_radix(input, 2)?,
                };
                Ok(this)
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

    fn from_le_bytes(bytes: &[u8]) -> Result<Self, FromBytesError> {
        match bytes.get(..32 / 8) {
            Some(slice) => Ok(Self::from_le_bytes(slice.try_into()?)),
            None => Err(FromBytesError::SliceIndexError),
        }
    }

    fn from_be_bytes(bytes: &[u8]) -> Result<Self, FromBytesError> {
        match bytes.get(..32 / 8) {
            Some(slice) => Ok(Self::from_be_bytes(slice.try_into()?)),
            None => Err(FromBytesError::SliceIndexError),
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

    fn to_bin_string(&self) -> String {
        "<no bin output>".into()
    }

    fn from_str(input: &str, format: Format) -> Result<Self, anyhow::Error> {
        let this = match format {
            Format::Decimal => input.parse()?,
            Format::Hex => bail!("Float doesn't support parsing hex"),
            Format::Bin => bail!("Float doesn't support parsing bin"),
        };
        Ok(this)
    }
}

impl NumBytesManip for f64 {
    type ToBytes = [u8; 8];

    fn label() -> &'static str {
        "f64"
    }

    fn from_le_bytes(bytes: &[u8]) -> Result<Self, FromBytesError> {
        match bytes.get(..8) {
            Some(slice) => Ok(Self::from_le_bytes(slice.try_into()?)),
            None => Err(FromBytesError::SliceIndexError),
        }
    }

    fn from_be_bytes(bytes: &[u8]) -> Result<Self, FromBytesError> {
        match bytes.get(..8) {
            Some(slice) => Ok(Self::from_be_bytes(slice.try_into()?)),
            None => Err(FromBytesError::SliceIndexError),
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

    fn to_bin_string(&self) -> String {
        "<no bin output>".into()
    }

    fn from_str(input: &str, format: Format) -> Result<Self, anyhow::Error> {
        let this = match format {
            Format::Decimal => input.parse()?,
            Format::Hex => bail!("Float doesn't support parsing hex"),
            Format::Bin => bail!("Float doesn't support parsing bin"),
        };
        Ok(this)
    }
}

impl<T: NumBytesManip> BytesManip for T {
    fn update_buf(buf: &mut String, data: &[u8], offset: usize, be: bool, format: Format) {
        if let Some(slice) = &data.get(offset..) {
            let result = if be {
                T::from_be_bytes(slice)
            } else {
                T::from_le_bytes(slice)
            };
            *buf = match result {
                Ok(value) => match format {
                    Format::Decimal => value.to_string(),
                    Format::Hex => value.to_hex_string(),
                    Format::Bin => value.to_bin_string(),
                },
                Err(e) => e.to_string(),
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
        format: Format,
        msg: &mut MessageDialog,
    ) -> Option<DamageRegion> {
        match Self::from_str(buf, format) {
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
            Err(e) => {
                msg.open(Icon::Error, "Convert error", e.to_string());
                None
            }
        }
    }
}

impl BytesManip for Ascii {
    fn update_buf(buf: &mut String, data: &[u8], offset: usize, _be: bool, _format: Format) {
        if let Some(slice) = &data.get(offset..) {
            let valid_ascii_end = find_valid_ascii_end(slice);
            match String::from_utf8(data[offset..offset + valid_ascii_end].to_vec()) {
                Ok(ascii) => *buf = ascii,
                Err(e) => *buf = format!("[ascii error]: {}", e),
            }
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
        _format: Format,
        msg: &mut MessageDialog,
    ) -> Option<DamageRegion> {
        let len = buf.len();
        let range = offset..offset + len;
        match data.get_mut(range.clone()) {
            Some(slice) => {
                slice.copy_from_slice(buf.as_bytes());
                Some(DamageRegion::Range(range))
            }
            None => {
                msg.open(
                    Icon::Error,
                    "Convert and write error",
                    "Failed to write data: Out of bounds",
                );
                None
            }
        }
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
    fn update_buf(buf: &mut String, data: &[u8], offset: usize, be: bool, format: Format);
    fn label() -> &'static str;
    fn convert_and_write(
        buf: &str,
        data: &mut [u8],
        offset: usize,
        be: bool,
        format: Format,
        msg: &mut MessageDialog,
    ) -> Option<DamageRegion>;
}

struct Ascii;

enum Action {
    GoToOffset(usize),
    AddDirty(DamageRegion),
    JumpForward(usize),
}

pub fn ui(ui: &mut Ui, app: &mut App, gui: &mut crate::gui::Gui, mouse_pos: ViewportVec) {
    if app.hex_ui.current_layout.is_null() {
        ui.label("No active layout");
        return;
    }
    let offset = match app.hex_ui.interact_mode {
        InteractMode::View if !ui.ctx().wants_pointer_input() => {
            if let Some((off, _view_idx)) = app.byte_offset_at_pos(mouse_pos.x, mouse_pos.y) {
                let mut add = 0;
                if gui.inspect_panel.offset_relative {
                    add = app.args.src.hard_seek.unwrap_or(0);
                }
                ui.link(format!("offset: {} (0x{:x})", off + add, off + add))
                    .context_menu(|ui| {
                        if ui.button("Copy to clipboard").clicked() {
                            ui.output().copied_text = format!("{:x}", off + add);
                            ui.close_menu();
                        }
                    });
                off
            } else {
                edit_offset(app, gui, ui)
            }
        }
        _ => edit_offset(app, gui, ui),
    };
    ui.checkbox(&mut gui.inspect_panel.offset_relative, "Relative offset")
        .on_hover_text("Offset relative to --hard-seek");
    if app.data.is_empty() {
        return;
    }
    if offset != gui.inspect_panel.prev_frame_inspect_offset
        || app.just_reloaded
        || gui.inspect_panel.changed_one
    {
        for thingy in &mut gui.inspect_panel.input_thingies {
            thingy.update(
                &app.data[..],
                offset,
                gui.inspect_panel.big_endian,
                gui.inspect_panel.format,
            );
        }
    }
    gui.inspect_panel.changed_one = false;
    let mut actions = Vec::new();
    for thingy in &mut gui.inspect_panel.input_thingies {
        ui.horizontal(|ui| {
            ui.label(thingy.label());
            if ui.button("📋").on_hover_text("copy to clipboard").clicked() {
                ui.output().copied_text = thingy.buf_mut().clone();
            }
            if ui.button("⬇").on_hover_text("go to offset").clicked() {
                let result: anyhow::Result<()> = try {
                    let offset = match gui.inspect_panel.format {
                        Format::Decimal => thingy.buf_mut().parse()?,
                        Format::Hex => usize::from_str_radix(thingy.buf_mut(), 16)?,
                        Format::Bin => usize::from_str_radix(thingy.buf_mut(), 2)?,
                    };
                    actions.push(Action::GoToOffset(offset));
                };
                msg_if_fail(result, "Failed to go to offset", &mut gui.msg_dialog);
            }
            if ui.button("➡").on_hover_text("jump forward").clicked() {
                let result: anyhow::Result<()> = try {
                    let offset = match gui.inspect_panel.format {
                        Format::Decimal => thingy.buf_mut().parse()?,
                        Format::Hex => usize::from_str_radix(thingy.buf_mut(), 16)?,
                        Format::Bin => usize::from_str_radix(thingy.buf_mut(), 2)?,
                    };
                    actions.push(Action::JumpForward(offset));
                };
                msg_if_fail(result, "Failed to jump forward", &mut gui.msg_dialog);
            }
        });
        if ui.text_edit_singleline(thingy.buf_mut()).lost_focus()
            && ui.input().key_pressed(egui::Key::Enter)
        {
            if let Some(range) = thingy.write_data(
                &mut app.data,
                offset,
                gui.inspect_panel.big_endian,
                gui.inspect_panel.format,
                &mut gui.msg_dialog,
            ) {
                gui.inspect_panel.changed_one = true;
                actions.push(Action::AddDirty(range));
            }
        }
    }
    ui.horizontal(|ui| {
        if ui
            .checkbox(&mut gui.inspect_panel.big_endian, "Big endian")
            .clicked()
        {
            // Changing this should refresh everything
            gui.inspect_panel.changed_one = true;
        }
        let prev_fmt = gui.inspect_panel.format;
        egui::ComboBox::new("format_combo", "format")
            .selected_text(gui.inspect_panel.format.label())
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut gui.inspect_panel.format,
                    Format::Decimal,
                    Format::Decimal.label(),
                );
                ui.selectable_value(
                    &mut gui.inspect_panel.format,
                    Format::Hex,
                    Format::Hex.label(),
                );
                ui.selectable_value(
                    &mut gui.inspect_panel.format,
                    Format::Bin,
                    Format::Bin.label(),
                );
            });

        if gui.inspect_panel.format != prev_fmt {
            // Changing the format should refresh everything
            gui.inspect_panel.changed_one = true;
        }
    });

    for action in actions {
        match action {
            Action::GoToOffset(offset) => {
                if gui.inspect_panel.offset_relative {
                    app.edit_state
                        .set_cursor(offset - app.args.src.hard_seek.unwrap_or(0));
                } else {
                    app.edit_state.set_cursor(offset);
                }
                app.center_view_on_offset(app.edit_state.cursor);
                app.hex_ui.flash_cursor();
            }
            Action::AddDirty(damage) => app.edit_state.widen_dirty_region(damage),
            Action::JumpForward(amount) => {
                app.edit_state.set_cursor(app.edit_state.cursor + amount);
                app.center_view_on_offset(app.edit_state.cursor);
                app.hex_ui.flash_cursor();
            }
        }
    }
    gui.inspect_panel.prev_frame_inspect_offset = offset;
}

fn edit_offset(app: &mut App, gui: &mut crate::gui::Gui, ui: &mut Ui) -> usize {
    let mut off = app.edit_state.cursor;
    if gui.inspect_panel.offset_relative {
        off += app.args.src.hard_seek.unwrap_or(0);
    }
    ui.link(format!("offset: {} ({:x}h)", off, off))
        .context_menu(|ui| {
            if ui.button("Copy to clipboard").clicked() {
                ui.output().copied_text = format!("{:x}", off);
                ui.close_menu();
            }
        });
    app.edit_state.cursor
}

fn find_valid_ascii_end(data: &[u8]) -> usize {
    // Don't try to take too many characters, as that degrades performance
    const MAX_TAKE: usize = 50;
    data.iter()
        .take(MAX_TAKE)
        .position(|&b| b == 0 || b > 127)
        .unwrap_or_else(|| std::cmp::min(MAX_TAKE, data.len()))
}
