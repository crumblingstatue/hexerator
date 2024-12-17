use {crate::app::command::CommandQueue, core::f32, egui::Color32, std::backtrace::Backtrace};

#[derive(Default)]
pub struct MessageDialog {
    title: String,
    desc: String,
    pub is_open: bool,
    icon: Icon,
    buttons_ui_fn: Option<Box<UiFn>>,
    pub backtrace: Option<Backtrace>,
    show_backtrace: bool,
}

#[derive(Default)]
pub enum Icon {
    #[default]
    None,
    Info,
    Warn,
    Error,
}

pub(crate) type UiFn = dyn FnMut(&mut egui::Ui, &mut MessageDialog, &mut CommandQueue);

// Colors and icon text are copied from egui-toast, for visual consistency
// https://github.com/urholaukkarinen/egui-toast
impl Icon {
    fn color(&self) -> Color32 {
        match self {
            Icon::None => Color32::default(),
            Icon::Info => Color32::from_rgb(0, 155, 255),
            Icon::Warn => Color32::from_rgb(255, 212, 0),
            Icon::Error => Color32::from_rgb(255, 32, 0),
        }
    }
    fn utf8(&self) -> &'static str {
        match self {
            Icon::None => "",
            Icon::Info => "ℹ",
            Icon::Warn => "⚠",
            Icon::Error => "❗",
        }
    }
    fn hover_text(&self) -> String {
        let label = match self {
            Icon::None => "",
            Icon::Info => "Info",
            Icon::Warn => "Warning",
            Icon::Error => "Error",
        };
        format!("{label}\n\nClick to copy message to clipboard")
    }
    fn is_set(&self) -> bool {
        !matches!(self, Self::None)
    }
}

impl MessageDialog {
    pub(crate) fn open(&mut self, icon: Icon, title: impl Into<String>, desc: impl Into<String>) {
        self.title = title.into();
        self.desc = desc.into();
        self.icon = icon;
        self.is_open = true;
        self.buttons_ui_fn = None;
    }
    pub(crate) fn custom_button_row_ui(&mut self, f: Box<UiFn>) {
        self.buttons_ui_fn = Some(f);
    }
    pub(crate) fn show(
        &mut self,
        ctx: &egui::Context,
        cb: &mut arboard::Clipboard,
        cmd: &mut CommandQueue,
    ) {
        if !self.is_open {
            return;
        }
        egui::Modal::new("msg_dialog_popup".into()).show(ctx, |ui| {
            ui.heading(&self.title);
            ui.vertical_centered_justified(|ui| {
                ui.horizontal(|ui| {
                    if self.icon.is_set()
                        && ui
                            .add(
                                egui::Label::new(
                                    egui::RichText::new(self.icon.utf8())
                                        .color(self.icon.color())
                                        .size(32.0),
                                )
                                .sense(egui::Sense::click()),
                            )
                            .on_hover_text(self.icon.hover_text())
                            .clicked()
                    {
                        if let Err(e) = cb.set_text(self.desc.clone()) {
                            gamedebug_core::per!("Clipboard set error: {e:?}");
                        }
                    }
                    ui.label(&self.desc);
                });
                if let Some(bt) = &self.backtrace {
                    ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                        ui.checkbox(&mut self.show_backtrace, "Show backtrace");
                        if self.show_backtrace {
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
                let mut buttons_ui_fn = self.buttons_ui_fn.take();
                match &mut buttons_ui_fn {
                    Some(f) => f(ui, self, cmd),
                    None => {
                        if ui.button("Ok").clicked() || enter_pressed || esc_pressed {
                            self.backtrace = None;
                            self.is_open = false;
                        }
                    }
                }
                self.buttons_ui_fn = buttons_ui_fn;
            });
        });
    }
}
