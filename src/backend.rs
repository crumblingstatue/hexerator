use crate::gui::message_dialog::{Icon, MessageDialog};

#[cfg(feature = "backend-sfml")]
mod sfml;

pub fn get_clipboard_string(msg: &mut MessageDialog) -> String {
    match arboard::Clipboard::new() {
        Ok(mut clip) => match clip.get_text() {
            Ok(text) => text,
            Err(e) => {
                msg.open(
                    Icon::Error,
                    "Failed to get text from clipboard",
                    e.to_string(),
                );
                String::new()
            }
        },
        Err(e) => {
            msg.open(Icon::Error, "Failed to access clipboard", e.to_string());
            String::new()
        }
    }
}
