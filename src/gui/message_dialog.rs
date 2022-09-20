use {egui_modal::Modal, egui_sfml::egui};

#[derive(Default)]
pub struct MessageDialog {
    title: String,
    desc: String,
    modal: Option<Modal>,
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
                    ui.label(&self.desc);
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
