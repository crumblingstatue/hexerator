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
#[derive(Default, Serialize, Deserialize)]
pub struct Meta {
    pub regions: RegionMap,
    pub perspectives: PerspectiveMap,
    pub views: ViewMap,
    pub layouts: LayoutMap,
    pub bookmarks: Vec<Bookmark>,
    pub misc: Misc,
}

/// Misc information that's worth saving
#[derive(Serialize, Deserialize)]
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

const DEFAULT_CODE: &str = r#"-- Return a byte based on offset `off` and the current byte value `b`
function(off, b)
   return off % 256
end"#;

impl Meta {
    /// Init required after deserializing
    pub fn post_load_init(&mut self) {
        for view in self.views.values_mut() {
            // Needed to initialize edit buffers, etc.
            view.view.adjust_state_to_kind();
        }
    }
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
