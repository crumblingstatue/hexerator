use {
    super::{WinCtx, WindowOpen},
    crate::{
        layout::Layout,
        meta::{
            perspective::Perspective, LayoutKey, NamedRegion, NamedView, PerspectiveKey, RegionKey,
            ViewKey,
        },
    },
    itertools::{EitherOrBoth, Itertools},
    slotmap::SlotMap,
    std::fmt::Debug,
};

#[derive(Default)]
pub struct MetaDiffWindow {
    pub open: WindowOpen,
}
impl super::Window for MetaDiffWindow {
    fn ui(&mut self, WinCtx { ui, app, .. }: WinCtx) {
        let this = &mut app.meta_state.meta;
        let clean = &app.meta_state.clean_meta;
        ui.heading("Regions");
        diff_slotmap(ui, &mut this.low.regions, &clean.low.regions);
        ui.heading("Perspectives");
        diff_slotmap(ui, &mut this.low.perspectives, &clean.low.perspectives);
        ui.heading("Views");
        diff_slotmap(ui, &mut this.views, &clean.views);
        ui.heading("Layouts");
        diff_slotmap(ui, &mut this.layouts, &clean.layouts);
    }

    fn title(&self) -> &str {
        "Diff against clean meta"
    }
}

trait SlotmapDiffItem: PartialEq + Eq + Clone + Debug {
    type Key: slotmap::Key;
    type SortKey: Ord;
    fn label(&self) -> &str;
    fn sort_key(&self) -> Self::SortKey;
}

impl SlotmapDiffItem for NamedRegion {
    type Key = RegionKey;

    fn label(&self) -> &str {
        &self.name
    }

    type SortKey = usize;

    fn sort_key(&self) -> Self::SortKey {
        self.region.begin
    }
}

impl SlotmapDiffItem for Perspective {
    type Key = PerspectiveKey;

    type SortKey = String;

    fn label(&self) -> &str {
        &self.name
    }

    fn sort_key(&self) -> Self::SortKey {
        self.name.clone()
    }
}

impl SlotmapDiffItem for NamedView {
    type Key = ViewKey;

    type SortKey = String;

    fn label(&self) -> &str {
        &self.name
    }

    fn sort_key(&self) -> Self::SortKey {
        self.name.clone()
    }
}

impl SlotmapDiffItem for Layout {
    type Key = LayoutKey;

    type SortKey = String;

    fn label(&self) -> &str {
        &self.name
    }

    fn sort_key(&self) -> Self::SortKey {
        self.name.to_owned()
    }
}

fn diff_slotmap<I: SlotmapDiffItem>(
    ui: &mut egui::Ui,
    this: &mut SlotMap<I::Key, I>,
    clean: &SlotMap<I::Key, I>,
) {
    let mut this_keys: Vec<_> = this.keys().collect();
    this_keys.sort_by_key(|&k| this[k].sort_key());
    let mut clean_keys: Vec<_> = clean.keys().collect();
    clean_keys.sort_by_key(|&k| clean[k].sort_key());
    let mut any_changed = false;
    for zip_item in this_keys.into_iter().zip_longest(clean_keys) {
        match zip_item {
            EitherOrBoth::Both(this_key, clean_key) => {
                if this_key != clean_key {
                    ui.label("-");
                    any_changed = true;
                    continue;
                }
                let this_item = &this[this_key];
                let clean_item = &clean[clean_key];
                if this_item != clean_item {
                    any_changed = true;
                    ui.label(format!(
                        "{}: {:?}\n=>\n{:?}",
                        this_item.label(),
                        this_item,
                        clean_item
                    ));
                }
            }
            EitherOrBoth::Left(this_key) => {
                any_changed = true;
                ui.label(format!("New {}", this[this_key].label()));
            }
            EitherOrBoth::Right(clean_key) => {
                any_changed = true;
                ui.label(format!("Deleted {}", clean[clean_key].label()));
            }
        }
    }
    if any_changed {
        if ui.button("Restore").clicked() {
            this.clone_from(clean);
        }
    } else {
        ui.label("No changes");
    }
}
