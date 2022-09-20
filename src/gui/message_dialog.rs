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
}

#[derive(Default)]
enum Icon {
    #[default]
    None,
    Info,
    Success,
    Warning,
    Error,
}

// Colors and icon text are copied from egui-toast, for visual consistency
// https://github.com/urholaukkarinen/egui-toast
impl Icon {
    fn color(&self) -> Color32 {
        match self {
            Icon::None => Color32::default(),
            Icon::Info => Color32::from_rgb(0, 155, 255),
            Icon::Success => Color32::from_rgb(0, 255, 32),
            Icon::Warning => Color32::from_rgb(255, 212, 0),
            Icon::Error => Color32::from_rgb(255, 32, 0),
        }
    }
    fn utf8(&self) -> &'static str {
        match self {
            Icon::None => "",
            Icon::Info => "ℹ",
            Icon::Success => "✔",
            Icon::Warning => "⚠",
            Icon::Error => "❗",
        }
    }
    fn hover_text(&self) -> String {
        let label = match self {
            Icon::None => "",
            Icon::Info => "Info",
            Icon::Success => "Success",
            Icon::Warning => "Warning",
            Icon::Error => "Error",
        };
        format!("{label}\n\nClick to copy message to clipboard")
    }
    fn is_set(&self) -> bool {
        !matches!(self, Self::None)
    }
}

impl MessageDialog {
    pub(crate) fn info(
        &mut self,
        ui: &mut egui::Ui,
        title: impl Into<String>,
        desc: impl Into<String>,
    ) {
        self.title = title.into();
        self.desc = desc.into();
        self.icon = Icon::Info;
        let modal = self
            .modal
            .get_or_insert_with(|| Modal::new(ui.ctx(), "modal_message_dialog"));
        modal.open();
    }

    pub(crate) fn show(&self) {
        if let Some(modal) = &self.modal {
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
                            ui.output().copied_text = self.desc.clone();
                        }
                        ui.label(&self.desc);
                    });
                    let (enter_pressed, esc_pressed) = {
                        let input = ui.input();
                        (
                            input.key_pressed(egui::Key::Enter),
                            input.key_pressed(egui::Key::Escape),
                        )
                    };
                    if ui.button("Ok").clicked() || enter_pressed || esc_pressed {
                        modal.close();
                    }
                });
            });
        }
    }
}
