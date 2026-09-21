/*! Folder, envelope, and message loading plus item actions. */

use crate::keys::View;
use crate::mail::types::FolderRole;

use super::model::{App, PAGE_SIZE};

impl App {
    pub(crate) fn load_accounts(&mut self) {
        self.loading = true;
        self.worker.fetch_accounts();
    }

    pub(crate) fn load_folders(&mut self) {
        self.loading = true;
        self.worker.fetch_folders(self.acct_owned());
    }

    pub(crate) fn load_envelopes(&mut self) {
        self.loading = true;
        self.worker
            .start_watching(self.acct_owned(), self.current_folder.clone());
        if self.threaded {
            self.worker.fetch_envelopes_threaded(
                self.acct_owned(),
                self.current_folder.clone(),
                self.active_query.clone(),
            );
        } else {
            self.worker.fetch_envelopes(
                self.acct_owned(),
                self.current_folder.clone(),
                self.page,
                PAGE_SIZE,
                self.active_query.clone(),
            );
        }
    }

    pub fn refresh_envelopes(&mut self) {
        self.load_envelopes();
    }

    pub(crate) fn load_message(&mut self) {
        if let Some(id) = self.selected_envelope_id().map(|s| s.to_string()) {
            self.loading = true;
            if self.current_folder_is_drafts() {
                self.pending_draft = Some((self.current_folder.clone(), id.clone()));
                self.worker.fetch_draft_template(
                    self.acct_owned(),
                    self.current_folder.clone(),
                    id,
                );
                return;
            }
            self.pending_message_id = Some(id.clone());
            self.worker
                .fetch_message(self.acct_owned(), self.current_folder.clone(), id);
        }
    }

    pub(crate) fn current_folder_is_drafts(&self) -> bool {
        self.folders
            .iter()
            .any(|folder| folder.name == self.current_folder && folder.role == FolderRole::Drafts)
    }

    // ── Mouse handling ───────────────────────────────────────────────

    pub(crate) fn delete(&mut self) {
        if let Some(id) = self.selected_envelope_id().map(|s| s.to_string()) {
            self.loading = true;
            self.pending_return_to_list = self.view == View::MessageView;
            self.pending_refresh_after_action = true;
            self.worker
                .delete_message(self.acct_owned(), self.current_folder.clone(), id);
        }
    }

    pub(crate) fn toggle_flag(&mut self) {
        if let Some(id) = self.selected_envelope_id().map(|s| s.to_string()) {
            let is_flagged = self
                .selected_envelope()
                .map(|e| e.is_flagged())
                .unwrap_or(false);
            self.loading = true;
            self.pending_refresh_after_action = true;
            if is_flagged {
                self.worker.flag_remove(
                    self.acct_owned(),
                    self.current_folder.clone(),
                    id,
                    "flagged".to_string(),
                );
            } else {
                self.worker.flag_add(
                    self.acct_owned(),
                    self.current_folder.clone(),
                    id,
                    "flagged".to_string(),
                );
            }
        }
    }

    pub(crate) fn download_attachments(&mut self) {
        if let Some(id) = self.selected_envelope_id().map(|s| s.to_string()) {
            self.loading = true;
            self.worker
                .download_attachments(self.acct_owned(), self.current_folder.clone(), id);
        }
    }

    pub(crate) fn toggle_thread(&mut self) {
        self.threaded = !self.threaded;
        self.page = 1;
        self.load_envelopes();
        if self.threaded {
            self.set_status("Threaded view.");
        } else {
            self.set_status("Flat view.");
        }
    }
}
