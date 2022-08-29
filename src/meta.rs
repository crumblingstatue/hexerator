pub mod perspective;

use serde::{Deserialize, Serialize};
use slotmap::{new_key_type, SlotMap};

use crate::{layout::Layout, region::Region, view::View};

use self::perspective::Perspective;

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

/// A bookmark for an offset in a file
#[derive(Serialize, Deserialize, Clone)]
pub struct Bookmark {
    /// Offset the bookmark applies to
    pub offset: usize,
    /// Short label
    pub label: String,
    /// Extended description
    pub desc: String,
}

/// Meta-information about a file that the user collects.
#[derive(Default)]
pub struct Meta {
    pub regions: RegionMap,
    pub perspectives: PerspectiveMap,
    pub view_map: ViewMap,
    pub view_layout_map: LayoutMap,
    pub bookmarks: Vec<Bookmark>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NamedRegion {
    pub name: String,
    pub region: Region,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NamedView {
    pub name: String,
    pub view: View,
}
