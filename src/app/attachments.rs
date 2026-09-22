/*! Attachment list overlay: browse, open, and save individual attachments. */

use crate::keys::View;
use crate::mail::attachments;

use super::model::{App, PendingOpenCommand};

impl App {
    /// Open the attachment list for the loaded message.
    pub(crate) fn open_attachments(&mut self) {
        let Some(message) = self.message_content.as_ref() else {
            return;
        };
        if message.attachments.is_empty() {
            self.set_status("This message has no attachments.");
            return;
        }
        self.attachment_index = 0;
        self.view = View::AttachmentList;
    }

    pub(crate) fn attachment_next(&mut self) {
        let count = self.attachment_count();
        if count > 0 {
            self.attachment_index = (self.attachment_index + 1).min(count - 1);
        }
    }

    pub(crate) fn attachment_prev(&mut self) {
        self.attachment_index = self.attachment_index.saturating_sub(1);
    }

    pub(crate) fn close_attachments(&mut self) {
        self.view = View::MessageView;
    }

    /// Open the selected attachment with the desktop handler.
    pub(crate) fn open_selected_attachment(&mut self) {
        let Some(path) = self.save_selected_attachment_file() else {
            return;
        };
        let program = std::env::var("FRANKING_OPENER").unwrap_or_else(|_| "xdg-open".to_string());
        self.pending_open_command = Some(PendingOpenCommand {
            program,
            args: vec![path.display().to_string()],
        });
    }

    /// Save the selected attachment to the download directory.
    pub(crate) fn save_selected_attachment(&mut self) {
        let Some(path) = self.save_selected_attachment_file() else {
            return;
        };
        self.set_status(&format!("Saved {}", path.display()));
    }

    /// Write the selected attachment and return the path it was written to.
    fn save_selected_attachment_file(&mut self) -> Option<std::path::PathBuf> {
        let payloads = self
            .message_content
            .as_ref()
            .map(attachments::payloads)
            .unwrap_or_default();
        let Some((name, bytes)) = payloads.into_iter().nth(self.attachment_index) else {
            self.set_status("No attachment is selected.");
            return None;
        };
        match attachments::save_attachments(vec![(name, bytes)], &attachments::downloads_dir()) {
            Ok(saved) => Some(std::path::PathBuf::from(saved)),
            Err(error) => {
                self.set_error(&format!("Could not save attachment: {error}"));
                None
            }
        }
    }

    /// Write the loaded message's source to the download directory.
    pub(crate) fn save_message(&mut self) {
        let Some(raw) = self
            .message_content
            .as_ref()
            .and_then(|message| message.raw.clone())
        else {
            self.set_error("No message source is available to save.");
            return;
        };
        let name = format!("{}.eml", self.message_file_stem());
        match attachments::save_attachments(vec![(name, raw)], &attachments::downloads_dir()) {
            Ok(path) => self.set_status(&format!("Saved {path}")),
            Err(error) => self.set_error(&format!("Could not save message: {error}")),
        }
    }

    fn attachment_count(&self) -> usize {
        self.message_content
            .as_ref()
            .map(|message| message.attachments.len())
            .unwrap_or(0)
    }
}

impl App {
    /// Preview a text attachment in a scrollable overlay.
    pub(crate) fn preview_attachment(&mut self) {
        let payloads = self
            .message_content
            .as_ref()
            .map(attachments::payloads)
            .unwrap_or_default();
        let Some((name, bytes)) = payloads.into_iter().nth(self.attachment_index) else {
            self.set_status("No attachment is selected.");
            return;
        };
        let content_type = self
            .message_content
            .as_ref()
            .and_then(|message| message.attachments.get(self.attachment_index))
            .and_then(|attachment| attachment.content_type.clone())
            .unwrap_or_default();
        if !is_textual(&name, &content_type) {
            self.set_status("This attachment is not text; press Enter to open it.");
            return;
        }
        let text = match String::from_utf8(bytes) {
            Ok(text) => text,
            Err(_) => {
                self.set_status("This attachment is not valid UTF-8 text.");
                return;
            }
        };
        let text: String = text
            .lines()
            .take(MAX_PREVIEW_LINES)
            .collect::<Vec<_>>()
            .join("\n");
        self.attachment_preview = Some((name, text));
        self.preview_scroll = 0;
        self.view = View::AttachmentView;
    }

    pub(crate) fn preview_scroll(&mut self, delta: i32) {
        if delta > 0 {
            self.preview_scroll = self.preview_scroll.saturating_add(1);
        } else {
            self.preview_scroll = self.preview_scroll.saturating_sub(1);
        }
    }

    pub(crate) fn close_attachment_preview(&mut self) {
        self.attachment_preview = None;
        self.view = View::AttachmentList;
    }
}

/// Lines shown from a previewed attachment.
const MAX_PREVIEW_LINES: usize = 2000;

/// Whether an attachment should be previewed as text.
fn is_textual(name: &str, content_type: &str) -> bool {
    if content_type.to_ascii_lowercase().starts_with("text/") {
        return true;
    }
    matches!(
        name.rsplit('.')
            .next()
            .map(str::to_ascii_lowercase)
            .as_deref(),
        Some(
            "txt" | "md" | "csv" | "json" | "log" | "ics" | "eml" | "xml" | "yaml" | "yml" | "toml"
        )
    )
}
