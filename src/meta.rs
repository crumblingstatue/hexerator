pub mod perspective;
pub mod region;
pub mod value_type;

use {
    self::{perspective::Perspective, region::Region, value_type::ValueType},
    crate::{layout::Layout, view::View},
    serde::{Deserialize, Serialize},
    slotmap::{new_key_type, SlotMap},
};

new_key_type! {
    pub struct PerspectiveKey;
    pub struct RegionKey;
    pub struct ViewKey;
    pub struct LayoutKey;
}

pub type PerspectiveMap = SlotMap<PerspectiveKey, Perspective>;
pub type RegionMap = SlotMap<RegionKey, NamedRegion>;
pub type ViewMap = SlotMap<ViewKey, NamedView>;
pub type LayoutMap = SlotMap<LayoutKey, Layout>;
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

    pub(crate) fn end_offset_of_view(&self, view: &mut View) -> usize {
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
}

impl Default for Misc {
    fn default() -> Self {
        Self {
            fill_lua_script: DEFAULT_CODE.into(),
        }
    }
}

const DEFAULT_CODE: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/lua/fill.lua"));

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
