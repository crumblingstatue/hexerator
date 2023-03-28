use {
    crate::gui::Dialog,
    egui::Button,
    iced_x86::{Decoder, Formatter, NasmFormatter},
};

pub struct X86AsmDialog {
    decoded: Vec<DecodedInstr>,
    bitness: u32,
}

impl X86AsmDialog {
    pub fn new() -> Self {
        Self {
            decoded: Vec::new(),
            bitness: 64,
        }
    }
}

impl Dialog for X86AsmDialog {
    fn title(&self) -> &str {
        "X86 assembly"
    }

    fn ui(
        &mut self,
        ui: &mut egui::Ui,
        app: &mut crate::app::App,
        _msg: &mut crate::gui::message_dialog::MessageDialog,
        _lua: &rlua::Lua,
        _font: &egui_sfml::sfml::graphics::Font,
        _events: &mut crate::event::EventQueue,
    ) -> bool {
        let mut retain = true;
        egui::ScrollArea::vertical()
            .max_height(320.0)
            .show(ui, |ui| {
                egui::Grid::new("asm_grid").num_columns(2).show(ui, |ui| {
                    for instr in &self.decoded {
                        let Some(sel_begin) = app.hex_ui.selection().map(|sel| sel.begin) else {
                            ui.label("No selection");
                            return;
                        };
                        let instr_off = instr.offset + sel_begin;
                        if ui.link(instr_off.to_string()).clicked() {
                            app.search_focus(instr_off);
                        }
                        ui.label(&instr.string);
                        ui.end_row();
                    }
                });
            });
        match app.hex_ui.selection() {
            Some(sel) => {
                if ui.button("Disassemble").clicked() {
                    self.decoded = disasm(&app.data[sel.begin..=sel.end], self.bitness);
                }
            }
            None => {
                ui.add_enabled(false, Button::new("Disassemble"));
            }
        }
        ui.label("Bitness");
        ui.radio_value(&mut self.bitness, 16, "16");
        ui.radio_value(&mut self.bitness, 32, "32");
        ui.radio_value(&mut self.bitness, 64, "64");
        if ui.button("Close").clicked() {
            retain = false;
        }
        retain
    }
}

struct DecodedInstr {
    string: String,
    offset: usize,
}

fn disasm(data: &[u8], bitness: u32) -> Vec<DecodedInstr> {
    let mut decoder = Decoder::new(bitness, data, 0);
    let mut fmt = NasmFormatter::default();
    let mut vec = Vec::new();
    while decoder.can_decode() {
        let offset = decoder.position();
        let instr = decoder.decode();
        let mut string = String::new();
        fmt.format(&instr, &mut string);
        vec.push(DecodedInstr { string, offset });
    }
    vec
}
