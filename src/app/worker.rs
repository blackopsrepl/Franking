/*! Background worker result polling and handlers. */

use crate::compose::populate_from_template;
use crate::keys::View;
use crate::mail::types::*;
use crate::mail::MessageDocument;
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
                    self.clear_autosave();
                    self.compose_state = None;
                    self.view = View::EnvelopeList;
                    if let Some((folder, id)) = self.pending_draft.take() {
                        self.worker.delete_message(self.acct_owned(), folder, id);
                    }
                    self.set_status(&msg);
                    self.refresh_envelopes();
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
                WorkerResult::MailboxChanged(_account, folder) => {
                    notify_new_mail(&folder);
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
        // Detect new mail by comparing unseen counts
        let old_unseen: usize = self.envelopes.iter().filter(|e| !e.is_seen()).count();
        let new_unseen: usize = envelopes.iter().filter(|e| !e.is_seen()).count();

        let was_populated = !self.envelopes.is_empty();
        let selection = self.envelope_state.selected();

        self.envelopes = envelopes;

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

    pub(crate) fn handle_message_loaded(&mut self, mut message: MessageDocument) {
        self.harvest_contacts_from_message(&message);
        let passphrase = self.crypto_passphrase.clone();
        let smime = super::smime::process_smime(&mut message);
        self.smime_signer = smime.as_ref().and_then(|outcome| outcome.untrusted.clone());
        self.pgp_status = super::pgp::process_pgp(&mut message, &passphrase)
            .or_else(|| smime.map(|outcome| outcome.status));

        self.message_content = Some(message);
        self.message_scroll = 0;
        self.loading = false;
        self.view = View::MessageView;
        // Mark as seen in local state
        if let Some(idx) = self.envelope_state.selected() {
            if let Some(env) = self.envelopes.get_mut(idx) {
                if !env.is_seen() {
                    env.flags.push("Seen".to_string());
                }
            }
        }
    }

    /// Parse From/To/Cc/Reply-To addresses from message headers and upsert them
    /// into the contacts DB. Errors are silently ignored (harvest is
    /// best-effort).
    pub(crate) fn harvest_contacts_from_message(&mut self, message: &MessageDocument) {
        if self.db.is_none() {
            return;
        }

        let mut addrs: Vec<(Option<String>, String)> = Vec::new();

        for header in message.header_fields() {
            let is_addr_header = header.name.eq_ignore_ascii_case("from")
                || header.name.eq_ignore_ascii_case("to")
                || header.name.eq_ignore_ascii_case("cc")
                || header.name.eq_ignore_ascii_case("reply-to");
            if is_addr_header {
                addrs.extend(crate::contacts::parse_address_list(&header.value));
            }
        }

        // Also harvest the envelope sender directly from the already-parsed list row.
        // Collect separately to avoid holding a borrow on self while calling upsert.
        let sender_str = self
            .selected_envelope()
            .map(|e| e.sender.display())
            .unwrap_or_default();
        if !sender_str.is_empty() {
            addrs.extend(crate::contacts::parse_address_list(&sender_str));
        }

        // Now borrow the connection and upsert all collected addresses.
        if let Some(ref conn) = self.db {
            for (name, email) in addrs {
                let _ = crate::contacts::upsert_harvested(conn, name.as_deref(), &email);
            }
        }
    }

    // ── Data loading (dispatches to worker) ─────────────────────────
}

/// Best-effort desktop notification for new mail (no-op when unavailable).
fn notify_new_mail(folder: &str) {
    use std::process::{Command, Stdio};

    let _ = Command::new("notify-send")
        .args(["SolverForge Mail", &format!("New mail in {folder}")])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
}
