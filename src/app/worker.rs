/*! Background worker result polling and handlers. */

use crate::compose::populate_from_template;
use crate::keys::View;
use crate::mail::types::*;
use crate::worker::WorkerResult;

use super::model::App;

impl App {
    pub(crate) fn poll_worker(&mut self) {
        for result in self.worker.drain() {
            match result {
                WorkerResult::Accounts(Ok(accounts)) => {
                    self.handle_accounts_loaded(accounts);
                }
                WorkerResult::Accounts(Err(e)) => {
                    self.loading = false;
                    self.set_error(&format!("Failed to load accounts: {e}"));
                }
                WorkerResult::Folders(Ok(folders)) => {
                    self.handle_folders_loaded(folders);
                }
                WorkerResult::Folders(Err(e)) => {
                    self.loading = false;
                    self.set_error(&format!("Failed to load folders: {e}"));
                }
                WorkerResult::Envelopes(Ok(envelopes)) => {
                    self.handle_envelopes_loaded(envelopes);
                }
                WorkerResult::Envelopes(Err(e)) => {
                    self.loading = false;
                    self.set_error(&format!("Failed to load envelopes: {e}"));
                }
                WorkerResult::Message(result) => match *result {
                    Ok(message) => self.handle_message_loaded(message),
                    Err(e) => {
                        self.loading = false;
                        self.set_error(&format!("Failed to read message: {e}"));
                    }
                },
                WorkerResult::ActionDone(Ok(msg)) => {
                    self.loading = false;
                    self.set_status(&msg);
                    self.worker
                        .fetch_folder_unread(self.acct_owned(), self.current_folder.clone());
                    if self.pending_return_to_list {
                        self.view = View::EnvelopeList;
                        self.pending_return_to_list = false;
                    }
                    if self.pending_refresh_after_action {
                        self.pending_refresh_after_action = false;
                        self.load_envelopes();
                    }
                    if self.pending_folder_refresh {
                        self.pending_folder_refresh = false;
                        self.load_folders();
                    }
                    if self.sieve.pending_refresh {
                        self.sieve.pending_refresh = false;
                        let account = self.acct_owned();
                        self.loading = true;
                        self.worker.fetch_sieve_scripts(account);
                    }
                }
                WorkerResult::ActionDone(Err(e)) => {
                    self.loading = false;
                    self.set_error(&format!("Error: {e}"));
                    self.pending_return_to_list = false;
                    self.pending_refresh_after_action = false;
                }
                WorkerResult::Discovered(config) => self.apply_discovered(config),
                WorkerResult::Outbox(result) => self.handle_outbox(result),
                WorkerResult::OAuthAuthorized(account, result) => {
                    let _ = account;
                    self.handle_oauth_authorized(result);
                }
                WorkerResult::SieveScripts(result) => self.handle_sieve_scripts(result),
                WorkerResult::SieveBody(name, result) => self.handle_sieve_body(name, result),
                WorkerResult::FolderUnread(folder_name, Ok(count)) => {
                    self.folder_unread.insert(folder_name, count);
                }
                WorkerResult::FolderUnread(_folder_name, Err(_)) => {
                    // Silently ignore unread count failures
                }
                WorkerResult::Template(Ok(template)) => {
                    self.loading = false;
                    self.handle_template_loaded(template);
                }
                WorkerResult::Template(Err(e)) => {
                    self.loading = false;
                    self.set_error(&format!("Template error: {e}"));
                    self.compose_state = None;
                }
                WorkerResult::SendDone(Ok(msg)) => {
                    self.loading = false;
                    let marker_error = self.complete_reply_marker().err();
                    self.clear_autosave();
                    self.compose_state = None;
                    self.view = View::EnvelopeList;
                    if let Some((account, folder, id)) = self.pending_draft.take() {
                        self.worker.delete_message(account, folder, id);
                    }
                    self.set_status(&msg);
                    self.refresh_envelopes();
                    if let Some(error) = marker_error {
                        self.set_error(&format!(
                            "Message sent, but reply queue update failed: {error}"
                        ));
                    }
                }
                WorkerResult::SendDone(Err(e)) => {
                    self.loading = false;
                    if let Some(ref mut cs) = self.compose_state {
                        cs.send_error = Some(e.to_string());
                    }
                    let account = self
                        .compose_state
                        .as_ref()
                        .and_then(|cs| cs.account.clone());
                    self.queue_failed_send(account);
                }
                WorkerResult::MailboxChanged(account, folder) => {
                    if self.notify_for_new_mail(account.as_deref(), &folder) {
                        super::notification_rules::notify_new_mail(&folder);
                    }
                    if folder == self.current_folder && !self.loading {
                        self.set_status("New mail arrived.");
                        self.load_envelopes();
                    }
                }
            }
        }
    }

    pub(crate) fn handle_template_loaded(&mut self, raw: String) {
        if let Some(ref mut cs) = self.compose_state {
            populate_from_template(cs, &raw);
            self.view = View::Compose;
        }
    }

    pub(crate) fn handle_accounts_loaded(&mut self, accounts: Vec<Account>) {
        // If no account was specified on the CLI, use the default.
        if self.account_name.is_none() {
            if let Some(account) = preferred_account(&accounts) {
                self.account_name = Some(account.name.clone());
            }
        }
        // Set account_index to match account_name
        if let Some(name) = &self.account_name {
            self.account_index = accounts.iter().position(|a| &a.name == name).unwrap_or(0);
        }
        self.accounts = accounts;
        // Chain: after accounts, load folders
        self.load_folders();
    }

    pub(crate) fn handle_folders_loaded(&mut self, folders: Vec<Folder>) {
        self.folders = folders;
        if self.accounts.len() > 1 {
            self.folders.insert(
                0,
                Folder {
                    name: super::model::UNIFIED_INBOX.to_string(),
                    desc: Some("All accounts".to_string()),
                    role: crate::mail::types::FolderRole::Inbox,
                    subscribed: None,
                },
            );
        }
        // Reset folder index to match current_folder
        self.folder_index = self
            .folders
            .iter()
            .position(|f| f.name == self.current_folder)
            .unwrap_or(0);
        // Fire off background unread count queries for each folder
        for folder in &self.folders {
            self.worker
                .fetch_folder_unread(self.acct_owned(), folder.name.clone());
        }
        // Chain: after folders, load envelopes
        self.load_envelopes();
    }

    pub(crate) fn handle_envelopes_loaded(&mut self, envelopes: Vec<Envelope>) {
        let envelopes = if let Some(lane) = self.triage_lane {
            let Some(conn) = self.db.as_ref() else {
                self.loading = false;
                self.set_error("Triage needs the local database.");
                return;
            };
            let mut matching = Vec::new();
            for envelope in envelopes {
                match crate::db::sender_routes::for_envelope(conn, &envelope) {
                    Ok(route) if route == lane => matching.push(envelope),
                    Ok(_) => {}
                    Err(error) => {
                        self.loading = false;
                        self.set_error(&format!("Triage error: {error}"));
                        return;
                    }
                }
            }
            matching
        } else {
            envelopes
        };
        // Detect new mail by comparing unseen counts
        let old_unseen: usize = self.envelopes.iter().filter(|e| !e.is_seen()).count();
        let new_unseen: usize = envelopes.iter().filter(|e| !e.is_seen()).count();

        let was_populated = !self.envelopes.is_empty();
        let selection = self.envelope_state.selected();

        self.envelopes = envelopes;
        if !self.threaded {
            let order = self.sort_order;
            order.apply(&mut self.envelopes);
        }

        if !self.envelopes.is_empty() {
            // Preserve selection position on auto-refresh if possible
            if was_populated {
                let idx = selection.unwrap_or(0).min(self.envelopes.len() - 1);
                self.envelope_state.select(Some(idx));
            } else {
                self.envelope_state.select(Some(0));
            }
        } else {
            self.envelope_state.select(None);
        }

        self.loading = false;
        self.ticks_since_refresh = 0;

        if was_populated && new_unseen > old_unseen {
            let diff = new_unseen - old_unseen;
            self.new_mail_count = diff;
            self.set_status(&format!(
                "{diff} new message{}.",
                if diff == 1 { "" } else { "s" }
            ));
        } else if self.envelopes.is_empty() {
            self.set_status("No messages.");
        } else if !was_populated {
            // Initial load, don't set "new mail" status
            self.status_message.clear();
        }
    }
}
