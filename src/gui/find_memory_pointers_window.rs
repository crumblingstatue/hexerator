use {
    super::{window_open::WindowOpen, Gui},
    crate::{app::App, event::EventQueue, shell::msg_fail},
    egui_extras::{Column, TableBuilder},
    egui_sfml::{egui, sfml::graphics::Font},
};

#[derive(Default)]
pub struct FindMemoryPointersWindow {
    pub open: WindowOpen,
    pointers: Vec<PtrEntry>,
}

struct PtrEntry {
    src_idx: usize,
    ptr: usize,
    range_idx: usize,
}

impl FindMemoryPointersWindow {
    pub fn ui(ui: &mut egui::Ui, gui: &mut Gui, app: &mut App, font: &Font, events: &EventQueue) {
        let Some(pid) = gui.open_process_window.selected_pid else {
            ui.label("No selected pid.");
            return;
        };
        let win = &mut gui.find_memory_pointers_window;
        if win.open.just_now() {
            for (i, wnd) in app
                .data
                .array_windows::<{ (usize::BITS / 8) as usize }>()
                .enumerate()
            {
                let ptr = usize::from_le_bytes(*wnd);
                if let Some(pos) = gui.open_process_window.map_ranges.iter().position(|range| {
                    range.is_read() && range.start() <= ptr && range.start() + range.size() >= ptr
                }) {
                    win.pointers.push(PtrEntry {
                        src_idx: i,
                        ptr,
                        range_idx: pos,
                    });
                }
            }
        }
        let mut action = Action::None;
        TableBuilder::new(ui)
            .column(Column::auto())
            .column(Column::auto())
            .column(Column::remainder())
            .striped(true)
            .header(20.0, |mut row| {
                row.col(|ui| {
                    ui.label("Location");
                });
                row.col(|ui| {
                    ui.label("Region");
                });
                row.col(|ui| {
                    ui.label("Pointer");
                });
            })
            .body(|body| {
                body.rows(20.0, win.pointers.len(), |mut row| {
                    let en = &win.pointers[row.index()];
                    row.col(|ui| {
                        if ui.link(format!("{:X}", en.src_idx)).clicked() {
                            action = Action::Goto(en.src_idx);
                        }
                    });
                    row.col(|ui| {
                        let range = &gui.open_process_window.map_ranges[en.range_idx];
                        ui.label(
                            range
                                .filename()
                                .map(|p| p.display().to_string())
                                .unwrap_or_else(|| String::from("<unnamed>")),
                        );
                    });
                    row.col(|ui| {
                        let range = &gui.open_process_window.map_ranges[en.range_idx];
                        if ui.link(format!("{:X}", en.ptr)).clicked() {
                            match app.load_proc_memory(
                                pid,
                                range.start(),
                                range.size(),
                                range.is_write(),
                                font,
                                &mut gui.msg_dialog,
                                events,
                            ) {
                                Ok(()) => action = Action::Goto(en.ptr - range.start()),
                                Err(e) => {
                                    msg_fail(&e, "failed to load proc memory", &mut gui.msg_dialog)
                                }
                            }
                        }
                    });
                });
            });
        match action {
            Action::Goto(off) => {
                app.center_view_on_offset(off);
                app.edit_state.set_cursor(off);
                app.hex_ui.flash_cursor();
            }
            Action::None => {}
        }
        win.open.post_ui();
    }
}

enum Action {
    Goto(usize),
    None,
}
