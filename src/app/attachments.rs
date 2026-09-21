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
        let program =
            std::env::var("SOLVERFORGE_OPENER").unwrap_or_else(|_| "xdg-open".to_string());
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

    fn attachment_count(&self) -> usize {
        self.message_content
            .as_ref()
            .map(|message| message.attachments.len())
            .unwrap_or(0)
    }
}
