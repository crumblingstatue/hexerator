use {
    self::{perspective::Perspective, region::Region, value_type::ValueType},
    crate::{layout::Layout, view::View},
    serde::{Deserialize, Serialize},
    slotmap::{new_key_type, SlotMap},
    std::{
        collections::HashMap,
        io::{ErrorKind, Write},
    },
};

pub mod perspective;
pub mod region;
pub mod value_type;

new_key_type! {
    pub struct PerspectiveKey;
    pub struct RegionKey;
    pub struct ViewKey;
    pub struct LayoutKey;
    pub struct ScriptKey;
}

pub type PerspectiveMap = SlotMap<PerspectiveKey, Perspective>;
pub type RegionMap = SlotMap<RegionKey, NamedRegion>;
pub type ViewMap = SlotMap<ViewKey, NamedView>;
pub type LayoutMap = SlotMap<LayoutKey, Layout>;
pub type ScriptMap = SlotMap<ScriptKey, Script>;
pub type Bookmarks = Vec<Bookmark>;

/// A bookmark for an offset in a file
#[derive(Serialize, Deserialize, Clone)]
pub struct Bookmark {
    /// Offset the bookmark applies to
    pub offset: usize,
    /// Short label
    pub label: String,
    /// Extended description
    pub desc: String,
    /// A bookmark can optionally have a type, which can be used to display its value, etc.
    #[serde(default)]
    pub value_type: ValueType,
}
impl Bookmark {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "Not much we can do about cast errors here"
    )]
    pub(crate) fn write_int(&self, mut data: &mut [u8], val: i64) -> std::io::Result<()> {
        match self.value_type {
            ValueType::None => Err(std::io::Error::new(
                ErrorKind::Other,
                "Bookmark doesn't have value type",
            )),
            ValueType::I8(_) => data.write_all(&(val as i8).to_ne_bytes()),
            ValueType::U8(_) => data.write_all(&(val as u8).to_ne_bytes()),
            ValueType::I16Le(_) => data.write_all(&(val as i16).to_le_bytes()),
            ValueType::U16Le(_) => data.write_all(&(val as u16).to_le_bytes()),
            ValueType::I16Be(_) => data.write_all(&(val as i16).to_be_bytes()),
            ValueType::U16Be(_) => data.write_all(&(val as u16).to_be_bytes()),
            ValueType::I32Le(_) => data.write_all(&(val as i32).to_le_bytes()),
            ValueType::U32Le(_) => data.write_all(&(val as u32).to_le_bytes()),
            ValueType::I32Be(_) => data.write_all(&(val as i32).to_be_bytes()),
            ValueType::U32Be(_) => data.write_all(&(val as u32).to_be_bytes()),
            ValueType::I64Le(_) => data.write_all(&(val).to_le_bytes()),
            ValueType::U64Le(_) => data.write_all(&(val as u64).to_le_bytes()),
            ValueType::I64Be(_) => data.write_all(&(val).to_be_bytes()),
            ValueType::U64Be(_) => data.write_all(&(val as u64).to_be_bytes()),
            ValueType::F32Le(_) => data.write_all(&(val as f32).to_le_bytes()),
            ValueType::F32Be(_) => data.write_all(&(val as f32).to_be_bytes()),
            ValueType::F64Le(_) => data.write_all(&(val as f64).to_le_bytes()),
            ValueType::F64Be(_) => data.write_all(&(val as f64).to_be_bytes()),
            ValueType::StringMap(_) => data.write_all(&(val as u8).to_ne_bytes()),
        }
    }
}

/// "Low" region of the meta, containing the least dependent data, like regions and perspectives
#[derive(Default, Serialize, Deserialize, Clone)]
pub struct MetaLow {
    pub regions: RegionMap,
    pub perspectives: PerspectiveMap,
}

impl MetaLow {
    pub(crate) fn start_offset_of_view(&self, view: &View) -> usize {
        let p = &self.perspectives[view.perspective];
        self.regions[p.region].region.begin
    }

    pub(crate) fn end_offset_of_view(&self, view: &View) -> usize {
        let p = &self.perspectives[view.perspective];
        self.regions[p.region].region.end
    }
}

/// Meta-information about a file that the user collects.
#[derive(Default, Serialize, Deserialize, Clone)]
pub struct Meta {
    pub low: MetaLow,
    pub views: ViewMap,
    pub layouts: LayoutMap,
    pub bookmarks: Bookmarks,
    pub misc: Misc,
    #[serde(default)]
    pub vars: HashMap<String, VarEntry>,
    #[serde(default)]
    pub scripts: ScriptMap,
    /// Script to execute when a document loads
    #[serde(default)]
    pub onload_script: Option<ScriptKey>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct VarEntry {
    pub val: VarVal,
    pub desc: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub enum VarVal {
    I64(i64),
    U64(u64),
}

pub(crate) fn find_most_specific_region_for_offset(
    regions: &RegionMap,
    off: usize,
) -> Option<RegionKey> {
    let mut most_specific = None;
    for (key, reg) in regions.iter() {
        if reg.region.contains(off) {
            match &mut most_specific {
                Some(most_spec_key) => {
                    // A region is more specific if it's smaller
                    let most_spec_reg = &regions[*most_spec_key];
                    if reg.region.len() < most_spec_reg.region.len() {
                        *most_spec_key = key;
                    }
                }
                None => most_specific = Some(key),
            }
        }
    }
    most_specific
}

/// Misc information that's worth saving
#[derive(Serialize, Deserialize, Clone)]
pub struct Misc {
    /// Lua script for the "Lua fill" feature.
    ///
    /// Worth saving because it can be used for binary file change testing, which can
    /// take a long time over many sessions.
    pub fill_lua_script: String,
    /// Lua script for the "execute script" feature.
    pub exec_lua_script: String,
}

impl Default for Misc {
    fn default() -> Self {
        Self {
            fill_lua_script: DEFAULT_FILL.into(),
            exec_lua_script: String::new(),
        }
    }
}

const DEFAULT_FILL: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/lua/fill.lua"));

impl Meta {
    /// Init required after deserializing
    pub fn post_load_init(&mut self) {
        for view in self.views.values_mut() {
            // Needed to initialize edit buffers, etc.
            view.view.adjust_state_to_kind();
        }
    }
    /// Returns offset and reference to a bookmark, if it corresponds to an offset
    pub fn bookmark_for_offset(
        meta_bookmarks: &Bookmarks,
        off: usize,
    ) -> Option<(usize, &Bookmark)> {
        meta_bookmarks
            .iter()
            .enumerate()
            .find(|(_i, b)| b.offset == off)
    }

    pub(crate) fn add_region_from_selection(&mut self, sel: Region) -> RegionKey {
        self.low
            .regions
            .insert(NamedRegion::new_from_selection(sel))
    }

    pub(crate) fn remove_view(&mut self, rem_key: ViewKey) {
        self.views.remove(rem_key);

        for layout in self.layouts.values_mut() {
            layout.remove_view(rem_key);
        }
    }

    pub(crate) fn bookmark_by_name_mut(&mut self, name: &str) -> Option<&mut Bookmark> {
        self.bookmarks.iter_mut().find(|bm| bm.label == name)
    }

    pub(crate) fn region_by_name_mut(&mut self, name: &str) -> Option<&mut NamedRegion> {
        self.low
            .regions
            .iter_mut()
            .find_map(|(_k, v)| (v.name == name).then_some(v))
    }
    /// Remove anything that contains dangling keys
    pub(crate) fn remove_dangling(&mut self) {
        self.low.perspectives.retain(|_k, v| {
            let mut retain = true;
            if !self.low.regions.contains_key(v.region) {
                eprintln!("Removed dangling perspective '{}'", v.name);
                retain = false;
            }
            retain
        });
        self.views.retain(|_k, v| {
            let mut retain = true;
            if !self.low.perspectives.contains_key(v.view.perspective) {
                eprintln!("Removed dangling view '{}'", v.name);
                retain = false;
            }
            retain
        });
        for layout in self.layouts.values_mut() {
            layout.remove_dangling(&self.views);
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct NamedRegion {
    pub name: String,
    pub region: Region,
    #[serde(default)]
    pub desc: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct NamedView {
    pub name: String,
    pub view: View,
}

impl NamedRegion {
    pub fn new_from_selection(sel: Region) -> Self {
        Self {
            name: format!("New ({}..={})", sel.begin, sel.end),
            region: sel,
            desc: String::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Script {
    pub name: String,
    pub desc: String,
    pub content: String,
}
