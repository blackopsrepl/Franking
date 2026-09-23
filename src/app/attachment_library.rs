/*! Cross-account attachment library overlay. */

use crate::keys::View;
use crate::mail::attachment_index::IndexedAttachment;

use super::model::App;

/// How many cached messages to scan when opening the library.
const SCAN_LIMIT: usize = 200;

/// Attachments indexed from locally cached mail.
#[derive(Default)]
pub struct AttachmentLibraryState {
    pub items: Vec<IndexedAttachment>,
    pub index: usize,
}

impl App {
    pub(crate) fn open_attachment_library(&mut self) {
        self.attachment_library.items.clear();
        self.attachment_library.index = 0;
        self.view = View::AttachmentLibrary;
        self.loading = true;
        self.set_status("Indexing cached attachments...");
        self.worker.fetch_attachment_library(SCAN_LIMIT);
    }

    /// Fill the overlay once the background scan finishes.
    pub(crate) fn handle_attachment_library(
        &mut self,
        result: Result<Vec<IndexedAttachment>, crate::mail::MailError>,
    ) {
        self.loading = false;
        match result {
            Ok(items) => {
                self.attachment_library.items = items;
                self.attachment_library.index = 0;
                if self.attachment_library.items.is_empty() {
                    self.set_status(
                        "No cached attachments yet. Open a message with attachments first.",
                    );
                } else {
                    self.status_message.clear();
                }
            }
            Err(error) => self.set_error(&format!("Could not index attachments: {error}")),
        }
    }

    pub(crate) fn attachment_library_next(&mut self) {
        let len = self.attachment_library.items.len();
        if len > 0 {
            self.attachment_library.index = (self.attachment_library.index + 1).min(len - 1);
        }
    }

    pub(crate) fn attachment_library_prev(&mut self) {
        self.attachment_library.index = self.attachment_library.index.saturating_sub(1);
    }

    /// Open the message that carries the highlighted attachment.
    pub(crate) fn open_attachment_library_item(&mut self) {
        let Some(item) = self
            .attachment_library
            .items
            .get(self.attachment_library.index)
        else {
            return;
        };
        let (account, folder, uid) = (item.account.clone(), item.folder.clone(), item.uid.clone());
        self.account_name = Some(account.clone());
        self.current_folder = folder.clone();
        self.triage_lane = None;
        self.followup_lane = None;
        self.active_query = None;
        self.pending_message_id = Some(uid.clone());
        self.loading = true;
        self.worker.fetch_message(Some(account), folder, uid);
    }

    pub(crate) fn close_attachment_library(&mut self) {
        self.view = View::EnvelopeList;
    }
}
