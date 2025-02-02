use {
    super::{WinCtx, WindowOpen},
    crate::{
        gui::egui_ui_ext::EguiResponseExt as _, meta::region::Region, shell::msg_if_fail,
        util::human_size,
    },
    egui_extras::{Column, TableBuilder},
};

pub struct ZeroPartition {
    pub open: WindowOpen,
    threshold: usize,
    regions: Vec<Region>,
    reload: bool,
}

impl Default for ZeroPartition {
    fn default() -> Self {
        Self {
            open: Default::default(),
            threshold: 4096,
            regions: Default::default(),
            reload: false,
        }
    }
}

impl super::Window for ZeroPartition {
    fn ui(&mut self, WinCtx { ui, app, gui, .. }: WinCtx) {
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
        ui.horizontal(|ui| {
            ui.label("Threshold");
            ui.add(egui::DragValue::new(&mut self.threshold));
            if ui.button("Go").clicked() {
                if self.reload {
                    msg_if_fail(app.reload(), "Failed to reload", &mut gui.msg_dialog);
                }
                self.regions = zero_partition(&app.data, self.threshold);
            }
            ui.checkbox(&mut self.reload, "reload")
                .on_hover_text("Auto reload data before partitioning");
            if !self.regions.is_empty() {
                ui.label(format!("{} results", self.regions.len()));
            }
        });
        if self.regions.is_empty() {
            return;
        }
        ui.separator();
        TableBuilder::new(ui)
            .columns(Column::auto(), 4)
            .auto_shrink([false, true])
            .striped(true)
            .header(24.0, |mut row| {
                row.col(|ui| {
                    if ui.button("begin").clicked() {
                        self.regions.sort_by_key(|r| r.begin);
                    }
                });
                row.col(|ui| {
                    if ui.button("end").clicked() {
                        self.regions.sort_by_key(|r| r.end);
                    }
                });
                row.col(|ui| {
                    if ui.button("size").clicked() {
                        self.regions.sort_by_key(|r| r.len());
                    }
                });
            })
            .body(|body| {
                body.rows(24.0, self.regions.len(), |mut row| {
                    let reg = &self.regions[row.index()];
                    if reg.contains(app.edit_state.cursor) {
                        row.set_selected(true);
                    }
                    row.col(|ui| {
                        if ui
                            .link(reg.begin.to_string())
                            .on_hover_text_deferred(|| human_size(reg.begin))
                            .clicked()
                        {
                            app.search_focus(reg.begin);
                        }
                    });
                    row.col(|ui| {
                        if ui
                            .link(reg.end.to_string())
                            .on_hover_text_deferred(|| human_size(reg.end))
                            .clicked()
                        {
                            app.search_focus(reg.end);
                        }
                    });
                    row.col(|ui| {
                        ui.label(reg.len().to_string())
                            .on_hover_text_deferred(|| human_size(reg.len()));
                    });
                    row.col(|ui| {
                        if ui.button("Select").clicked() {
                            app.hex_ui.select_a = Some(reg.begin);
                            app.hex_ui.select_b = Some(reg.end);
                        }
                    });
                });
            });
    }

    fn title(&self) -> &str {
        "Zero partition"
    }
}

fn zero_partition(data: &[u8], threshold: usize) -> Vec<Region> {
    if data.is_empty() {
        return Vec::new();
    }
    let mut regions = Vec::new();
    let mut reg = Region { begin: 0, end: 0 };
    let mut in_zero = if threshold == 1 { data[0] == 0 } else { false };
    let mut zero_counter = 0;
    for (i, &byte) in data.iter().enumerate() {
        if byte == 0 {
            zero_counter += 1;
            if zero_counter == threshold {
                if i > threshold && !in_zero {
                    reg.end = i.saturating_sub(threshold);
                    regions.push(reg);
                }
                in_zero = true;
            }
        } else {
            zero_counter = 0;
            if in_zero {
                in_zero = false;
                reg.begin = i;
            }
        }
    }
    if !in_zero {
        reg.end = data.len() - 1;
        regions.push(reg);
    }
    regions
}

#[test]
fn test_zero_partition() {
    assert_eq!(
        zero_partition(&[1, 1, 0, 0, 0, 1, 2, 3], 3),
        vec![Region { begin: 0, end: 1 }, Region { begin: 5, end: 7 }]
    );
    assert_eq!(
        zero_partition(&[1, 1, 0, 0, 0, 1, 2, 3], 4),
        vec![Region { begin: 0, end: 7 }]
    );
    assert_eq!(
        zero_partition(
            &[0, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 0, 0, 0, 1],
            3
        ),
        vec![
            Region { begin: 0, end: 4 },
            Region { begin: 11, end: 14 },
            Region { begin: 18, end: 18 }
        ]
    );
    assert_eq!(
        zero_partition(
            &[0, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 0, 0, 0, 1],
            1
        ),
        vec![
            Region { begin: 1, end: 4 },
            Region { begin: 11, end: 14 },
            Region { begin: 18, end: 18 }
        ]
    );
    // head and tail that exceed threshold
    assert_eq!(
        zero_partition(
            &[
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 2, 3, 0, 0, 0, 0, 4, 5, 6, 0, 0, 0, 0, 0, 0
            ],
            4
        ),
        vec![Region { begin: 10, end: 12 }, Region { begin: 17, end: 19 },]
    );
}
