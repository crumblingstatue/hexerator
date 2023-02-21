use {
    egui_modal::Modal,
    egui_sfml::{egui, egui::Color32},
};

#[derive(Default)]
pub struct MessageDialog {
    title: String,
    desc: String,
    modal: Option<Modal>,
    icon: Icon,
    /// If set, it will open modal on next call of show()
    open_modal: bool,
    buttons_ui_fn: Option<Box<UiFn>>,
}

#[derive(Default)]
pub enum Icon {
    #[default]
    None,
    Info,
    Warn,
    Error,
}

pub(crate) type UiFn = dyn FnMut(&mut egui::Ui, &Modal);

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
        self.open_modal = true;
        self.buttons_ui_fn = None;
    }
    pub(crate) fn custom_button_row_ui(&mut self, f: Box<UiFn>) {
        self.buttons_ui_fn = Some(f);
    }

    pub(crate) fn show(&mut self, ctx: &egui::Context, cb: &mut arboard::Clipboard) {
        let modal = self
            .modal
            .get_or_insert_with(|| Modal::new(ctx, "modal_message_dialog"));
        if self.open_modal {
            modal.open();
            self.open_modal = false;
        }
        modal.show(|ui| {
            modal.title(ui, &self.title);
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
                match &mut self.buttons_ui_fn {
                    Some(f) => f(ui, modal),
                    None => {
                        if ui.button("Ok").clicked() || enter_pressed || esc_pressed {
                            modal.close();
                        }
                    }
                }
            });
        });
    }
}
