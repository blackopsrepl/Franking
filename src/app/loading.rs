/*! Folder, envelope, and message loading plus item actions. */

use crate::keys::View;
use crate::mail::types::FolderRole;

use super::model::App;
use super::undo::UndoOp;

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
        if self.current_folder == super::model::UNIFIED_INBOX {
            self.worker.fetch_all_inboxes("INBOX".to_string());
            return;
        }
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
                self.page_size,
                self.active_query.clone(),
                self.sort_order,
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
                self.pending_draft = Some((self.selected_folder(), id.clone()));
                self.worker.fetch_draft_template(
                    self.selected_account(),
                    self.selected_folder(),
                    id,
                );
                return;
            }
            self.pending_message_id = Some(id.clone());
            self.worker
                .fetch_message(self.selected_account(), self.selected_folder(), id);
        }
    }

    /// Account for the selected envelope, falling back to the current account.
    pub(crate) fn selected_account(&self) -> Option<String> {
        self.selected_envelope()
            .and_then(|envelope| envelope.account.clone())
            .or_else(|| self.acct_owned())
    }

    /// Folder for the selected envelope, falling back to the current folder.
    pub(crate) fn selected_folder(&self) -> String {
        self.selected_envelope()
            .and_then(|envelope| envelope.folder.clone())
            .unwrap_or_else(|| self.current_folder.clone())
    }

    pub(crate) fn current_folder_is_drafts(&self) -> bool {
        self.folders
            .iter()
            .any(|folder| folder.name == self.current_folder && folder.role == FolderRole::Drafts)
    }

    // ── Mouse handling ───────────────────────────────────────────────

    pub(crate) fn delete(&mut self) {
        let ids = self.target_ids();
        let Some(id) = ids.first().cloned() else {
            return;
        };
        self.loading = true;
        self.pending_return_to_list = self.view == View::MessageView;
        self.pending_refresh_after_action = true;
        self.record_delete_undo(&ids);
        let account = self.selected_account();
        let folder = self.selected_folder();
        if ids.len() == 1 {
            self.worker.delete_message(account, folder, id);
        } else {
            self.selected.clear();
            self.worker.delete_messages(account, folder, ids);
        }
    }

    /// Deleting moves a message to Trash, so the reverse is a move back.
    fn record_delete_undo(&mut self, ids: &[String]) {
        let from = self.selected_folder();
        let Some(trash) = self.trash_folder() else {
            self.pending_undo = None;
            return;
        };
        if from.eq_ignore_ascii_case(&trash) {
            self.pending_undo = None;
            return;
        }
        self.pending_undo = Some(UndoOp::Move {
            from,
            to: trash,
            ids: ids.to_vec(),
        });
    }

    pub(crate) fn mark_folder_read(&mut self) {
        if self.current_folder == super::model::UNIFIED_INBOX {
            return;
        }
        self.loading = true;
        self.pending_refresh_after_action = true;
        self.worker
            .mark_folder_seen(self.acct_owned(), self.current_folder.clone());
    }

    pub(crate) fn sync_folder(&mut self) {
        self.loading = true;
        self.set_status("Caching folder for offline use...");
        self.worker
            .sync_folder(self.acct_owned(), self.current_folder.clone());
    }

    pub(crate) fn toggle_read(&mut self) {
        let ids = self.target_ids();
        let Some(id) = ids.first().cloned() else {
            return;
        };
        let is_seen = self
            .selected_envelope()
            .map(|envelope| envelope.is_seen())
            .unwrap_or(false);
        self.loading = true;
        self.pending_refresh_after_action = true;
        self.pending_undo = Some(UndoOp::Flag {
            folder: self.selected_folder(),
            ids: ids.clone(),
            flag: "seen".to_string(),
            added: !is_seen,
        });
        let account = self.selected_account();
        let folder = self.selected_folder();
        if ids.len() == 1 {
            if is_seen {
                self.worker
                    .flag_remove(account, folder, id, "seen".to_string());
            } else {
                self.worker
                    .flag_add(account, folder, id, "seen".to_string());
            }
        } else {
            self.selected.clear();
            self.worker
                .flag_messages(account, folder, ids, "seen".to_string(), !is_seen);
        }
    }

    pub(crate) fn toggle_flag(&mut self) {
        let ids = self.target_ids();
        let Some(id) = ids.first().cloned() else {
            return;
        };
        let is_flagged = self
            .selected_envelope()
            .map(|e| e.is_flagged())
            .unwrap_or(false);
        self.loading = true;
        self.pending_refresh_after_action = true;
        self.pending_undo = Some(UndoOp::Flag {
            folder: self.selected_folder(),
            ids: ids.clone(),
            flag: "flagged".to_string(),
            added: !is_flagged,
        });
        let account = self.selected_account();
        let folder = self.selected_folder();
        if ids.len() == 1 {
            if is_flagged {
                self.worker
                    .flag_remove(account, folder, id, "flagged".to_string());
            } else {
                self.worker
                    .flag_add(account, folder, id, "flagged".to_string());
            }
        } else {
            self.selected.clear();
            self.worker
                .flag_messages(account, folder, ids, "flagged".to_string(), !is_flagged);
        }
    }

    pub(crate) fn download_attachments(&mut self) {
        if let Some(id) = self.selected_envelope_id().map(|s| s.to_string()) {
            self.loading = true;
            self.worker
                .download_attachments(self.selected_account(), self.selected_folder(), id);
        }
    }

    /// Download every attachment of the selected message as one archive.
    pub(crate) fn download_attachments_zip(&mut self) {
        if let Some(id) = self.selected_envelope_id().map(|s| s.to_string()) {
            self.loading = true;
            self.worker.download_attachments_zip(
                self.selected_account(),
                self.selected_folder(),
                id,
            );
        }
    }

    pub(crate) fn toggle_thread(&mut self) {
        self.threaded = !self.threaded;
        if !self.threaded {
            self.clear_collapsed_threads();
        }
        self.page = 1;
        self.load_envelopes();
        if self.threaded {
            self.set_status("Threaded view.");
        } else {
            self.set_status("Flat view.");
        }
    }
}
