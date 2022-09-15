use super::window_open::WindowOpen;

#[derive(Default)]
pub struct HelpWindow {
    pub open: WindowOpen,
    pub topic_index: usize,
}

struct Topic {
    name: &'static str,
    contents: &'static str,
    id: &'static str,
}

macro_rules! topic {
    ($id: literal, $name: literal) => {
        Topic {
            name: $name,
            contents: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/help/", $id, ".md")),
            id: $id,
        }
    };
}

const TOPICS: [Topic; 4] = [
    topic!("index", "Hexerator"),
    topic!("keys", "Keys"),
    topic!("modal-editing", "Modal editing"),
    topic!("perspective", "Perspective"),
];

impl HelpWindow {
    pub(crate) fn ui(ui: &mut egui_sfml::egui::Ui, gui: &mut crate::gui::Gui) {
        ui.set_min_width(768.0);
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                for (i, topic) in TOPICS.iter().enumerate() {
                    if ui
                        .selectable_label(i == gui.help_window.topic_index, topic.name)
                        .clicked()
                    {
                        gui.help_window.topic_index = i;
                    }
                }
            });
            ui.separator();
            if let Some(url) = egui_easy_mark_standalone::easy_mark(
                ui,
                TOPICS[gui.help_window.topic_index].contents,
            ) {
                for (i, topic) in TOPICS.iter().enumerate() {
                    if url == topic.id {
                        gui.help_window.topic_index = i;
                    }
                }
            }
        });
    }
}
