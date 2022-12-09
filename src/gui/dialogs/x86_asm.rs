use {
    crate::gui::Dialog,
    egui::Button,
    iced_x86::{Decoder, Formatter, NasmFormatter},
};

pub struct X86AsmDialog {
    asm_buf: String,
    bitness: u32,
}

impl X86AsmDialog {
    pub fn new() -> Self {
        Self {
            asm_buf: String::new(),
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
                ui.add(egui::TextEdit::multiline(&mut self.asm_buf).code_editor());
            });
        match app.hex_ui.selection() {
            Some(sel) => {
                if ui.button("Disassemble").clicked() {
                    self.asm_buf = disasm(&app.data[sel.begin..=sel.end], self.bitness);
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

fn disasm(data: &[u8], bitness: u32) -> String {
    let mut decoder = Decoder::new(bitness, data, 0);
    let mut fmt = NasmFormatter::default();
    let mut out = String::new();
    while decoder.can_decode() {
        let instr = decoder.decode();
        fmt.format(&instr, &mut out);
        out.push('\n');
    }
    out
}
