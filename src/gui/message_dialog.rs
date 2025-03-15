use {
    crate::app::command::CommandQueue,
    core::f32,
    egui::Color32,
    std::{backtrace::Backtrace, collections::VecDeque},
};

#[derive(Default)]
pub struct MessageDialog {
    payloads: VecDeque<Payload>,
}

pub struct Payload {
    pub title: String,
    pub desc: String,
    pub icon: Icon,
    pub buttons_ui_fn: Option<Box<UiFn>>,
    pub backtrace: Option<Backtrace>,
    pub show_backtrace: bool,
    pub close: bool,
}

#[derive(Default)]
pub enum Icon {
    #[default]
    None,
    Info,
    Warn,
    Error,
}

pub(crate) type UiFn = dyn FnMut(&mut egui::Ui, &mut Payload, &mut CommandQueue);

// Colors and icon text are copied from egui-toast, for visual consistency
// https://github.com/urholaukkarinen/egui-toast
impl Icon {
    fn color(&self) -> Color32 {
        match self {
            Self::None => Color32::default(),
            Self::Info => Color32::from_rgb(0, 155, 255),
            Self::Warn => Color32::from_rgb(255, 212, 0),
            Self::Error => Color32::from_rgb(255, 32, 0),
        }
    }
    fn utf8(&self) -> &'static str {
        match self {
            Self::None => "",
            Self::Info => "ℹ",
            Self::Warn => "⚠",
            Self::Error => "❗",
        }
    }
    fn hover_text(&self) -> String {
        let label = match self {
            Self::None => "",
            Self::Info => "Info",
            Self::Warn => "Warning",
            Self::Error => "Error",
        };
        format!("{label}\n\nClick to copy message to clipboard")
    }
    fn is_set(&self) -> bool {
        !matches!(self, Self::None)
    }
}

impl MessageDialog {
    pub(crate) fn open(&mut self, icon: Icon, title: impl Into<String>, desc: impl Into<String>) {
        self.payloads.push_back(Payload {
            title: title.into(),
            desc: desc.into(),
            icon,
            buttons_ui_fn: None,
            backtrace: None,
            show_backtrace: false,
            close: false,
        });
    }
    pub(crate) fn custom_button_row_ui(&mut self, f: Box<UiFn>) {
        if let Some(front) = self.payloads.front_mut() {
            front.buttons_ui_fn = Some(f);
        }
    }
    pub(crate) fn show(
        &mut self,
        ctx: &egui::Context,
        cb: &mut arboard::Clipboard,
        cmd: &mut CommandQueue,
    ) {
        let payloads_len = self.payloads.len();
        let Some(payload) = self.payloads.front_mut() else {
            return;
        };
        let mut close = false;
        egui::Modal::new("msg_dialog_popup".into()).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading(&payload.title);
                if payloads_len > 1 {
                    ui.label(format!("({} more)", payloads_len - 1));
                }
            });
            ui.vertical_centered_justified(|ui| {
                ui.horizontal(|ui| {
                    if payload.icon.is_set()
                        && ui
                            .add(
                                egui::Label::new(
                                    egui::RichText::new(payload.icon.utf8())
                                        .color(payload.icon.color())
                                        .size(32.0),
                                )
                                .sense(egui::Sense::click()),
                            )
                            .on_hover_text(payload.icon.hover_text())
                            .clicked()
                    {
                        if let Err(e) = cb.set_text(payload.desc.clone()) {
                            gamedebug_core::per!("Clipboard set error: {e:?}");
                        }
                    }
                    ui.label(&payload.desc);
                });
                if let Some(bt) = &payload.backtrace {
                    ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                        ui.checkbox(&mut payload.show_backtrace, "Show backtrace");
                        if payload.show_backtrace {
                            let bt = bt.to_string();
                            egui::ScrollArea::both().max_height(300.0).show(ui, |ui| {
                                ui.add(
                                    egui::TextEdit::multiline(&mut bt.as_str())
                                        .code_editor()
                                        .desired_width(f32::INFINITY),
                                );
                            });
                        }
                    });
                }
                let (enter_pressed, esc_pressed) = ui.input_mut(|inp| {
                    (
                        // Consume enter and escape, so when the dialog is closed
                        // using these keys, the normal UI won't receive these keys right away.
                        // Receiving the keys could for example cause a text parse box
                        // that parses on enter press to parse again right away with the
                        // same error when the message box is closed with enter.
                        inp.consume_key(egui::Modifiers::default(), egui::Key::Enter),
                        inp.consume_key(egui::Modifiers::default(), egui::Key::Escape),
                    )
                });
                let mut buttons_ui_fn = payload.buttons_ui_fn.take();
                match &mut buttons_ui_fn {
                    Some(f) => f(ui, payload, cmd),
                    None => {
                        if ui.button("Ok").clicked() || enter_pressed || esc_pressed {
                            payload.backtrace = None;
                            close = true;
                        }
                    }
                }
                payload.buttons_ui_fn = buttons_ui_fn;
            });
        });
        if close || payload.close {
            self.payloads.pop_front();
        }
    }
    pub fn set_backtrace_for_top(&mut self, bt: Backtrace) {
        if let Some(front) = self.payloads.front_mut() {
            front.backtrace = Some(bt);
        }
    }
}
