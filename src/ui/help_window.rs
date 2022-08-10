use egui_sfml::egui::ScrollArea;

#[derive(Default)]
pub struct HelpWindow {
    pub open: bool,
    pub topic_index: usize,
}

struct Topic {
    name: &'static str,
    contents: &'static str,
}

const TOPICS: [Topic; 2] = [
    Topic {
        name: "Hexerator",
        contents: include_str!("../../help/index.md"),
    },
    Topic {
        name: "Keys",
        contents: include_str!("../../help/keys.md"),
    },
];

impl HelpWindow {
    pub(crate) fn ui(ui: &mut egui_sfml::egui::Ui, app: &mut crate::app::App) {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ScrollArea::vertical().show(ui, |ui| {
                    for (i, topic) in TOPICS.iter().enumerate() {
                        if ui
                            .selectable_label(i == app.ui.help_window.topic_index, topic.name)
                            .clicked()
                        {
                            app.ui.help_window.topic_index = i;
                        }
                    }
                });
            });
            ui.separator();
            if let Some(url) = egui_easy_mark_standalone::easy_mark(
                ui,
                TOPICS[app.ui.help_window.topic_index].contents,
            ) {
                match url {
                    "keys" => app.ui.help_window.topic_index = 1,
                    etc => eprintln!("Unhandled URL: {}", etc),
                }
            }
        });
    }
}
