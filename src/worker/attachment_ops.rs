/*! Indexing cached attachments off the UI thread. */

use crate::mail::MailError;

use super::dispatch::{Worker, WorkerResult};

impl Worker {
    /// Scan cached mail for attachments without blocking the interface.
    pub fn fetch_attachment_library(&self, limit: usize) {
        self.spawn(
            move |_service| {
                let conn = crate::db::open()
                    .map_err(|error| MailError::config_invalid(error.to_string()))?;
                crate::mail::attachment_index::list(&conn, limit)
                    .map_err(|error| MailError::config_invalid(error.to_string()))
            },
            WorkerResult::AttachmentLibrary,
        );
    }
}
