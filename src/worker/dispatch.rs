use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

use crate::mail::errors::MailResult;
use crate::mail::service::SendOptions;
use crate::mail::types::*;
use crate::mail::{default_mail_service, MailError, MailService, MessageDocument};

use super::watch::Watcher;

/// Messages sent from background threads back to the main App.
#[derive(Debug)]
pub enum WorkerResult {
    Accounts(Result<Vec<Account>, MailError>),
    Folders(Result<Vec<Folder>, MailError>),
    Envelopes(Result<Vec<Envelope>, MailError>),
    Message(Box<Result<MessageDocument, MailError>>),
    ActionDone(Result<String, MailError>),
    /// Unread count for a specific folder: (folder_name, count).
    FolderUnread(String, Result<usize, MailError>),
    /// A fetched compose/reply/forward template.
    Template(Result<String, MailError>),
    /// Result of sending a template.
    SendDone(Result<String, MailError>),
    /// The watched mailbox changed: (account, folder).
    MailboxChanged(Option<String>, String),
    /// Server-side Sieve scripts for an account.
    SieveScripts(Result<Vec<crate::mail::sieve::SieveScript>, MailError>),
    /// A fetched Sieve script: (name, source).
    SieveBody(String, Result<String, MailError>),
    /// Provider settings discovered for an email address.
    Discovered(Option<crate::mail::account_store::DiscoveredConfig>),
    /// Queued outgoing messages.
    Outbox(Result<Vec<crate::mail::outbox::OutboxItem>, MailError>),
    /// Result of an OAuth authorization attempt.
    OAuthAuthorized(String, Result<String, MailError>),
}

/// Lightweight handle for dispatching work to background threads.
/// Results are collected via try_recv() in the main loop.
pub struct Worker {
    pub(super) tx: mpsc::Sender<WorkerResult>,
    rx: mpsc::Receiver<WorkerResult>,
    pub(super) service: Arc<dyn MailService>,
    pub(super) watcher: Mutex<Option<Watcher>>,
}

impl Default for Worker {
    fn default() -> Self {
        Self::new()
    }
}

impl Worker {
    /// The mail service this worker dispatches to.
    ///
    /// Exposed so a caller can ask one quick question (such as who sent the
    /// newest message) without dispatching a job and waiting for the reply.
    pub fn service(&self) -> Arc<dyn MailService> {
        self.service.clone()
    }

    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            tx,
            rx,
            service: default_mail_service(),
            watcher: Mutex::new(None),
        }
    }

    /// Non-blocking: returns any completed results.
    pub fn try_recv(&self) -> Option<WorkerResult> {
        self.rx.try_recv().ok()
    }

    /// Drain all pending results.
    pub fn drain(&self) -> Vec<WorkerResult> {
        let mut results = Vec::new();
        while let Some(r) = self.try_recv() {
            results.push(r);
        }
        results
    }
    // ── Dispatch methods ────────────────────────────────────────────

    pub fn fetch_accounts(&self) {
        let tx = self.tx.clone();
        let service = self.service.clone();
        thread::spawn(move || {
            let result = service.list_accounts();
            let _ = tx.send(WorkerResult::Accounts(result));
        });
    }

    /// Subscribe or unsubscribe a folder, then reload the listing.
    pub fn set_folder_subscription(
        &self,
        account: Option<String>,
        folder: String,
        subscribed: bool,
    ) {
        let tx = self.tx.clone();
        let service = self.service.clone();
        thread::spawn(move || {
            let result = if subscribed {
                service.subscribe_folder(account.as_deref(), &folder)
            } else {
                service.unsubscribe_folder(account.as_deref(), &folder)
            };
            let message = result.map(|()| {
                if subscribed {
                    format!("Subscribed to {folder}.")
                } else {
                    format!("Unsubscribed from {folder}.")
                }
            });
            let _ = tx.send(WorkerResult::ActionDone(message));
        });
    }

    pub fn fetch_folders(&self, account: Option<String>) {
        let tx = self.tx.clone();
        let service = self.service.clone();
        thread::spawn(move || {
            let result = service.list_folders_detailed(account.as_deref());
            let _ = tx.send(WorkerResult::Folders(result));
        });
    }

    pub fn fetch_envelopes(
        &self,
        account: Option<String>,
        folder: String,
        page: usize,
        page_size: usize,
        query: Option<String>,
        order: crate::mail::sort::SortOrder,
    ) {
        let tx = self.tx.clone();
        let service = self.service.clone();
        thread::spawn(move || {
            let result = service.list_envelopes_sorted(
                account.as_deref(),
                &folder,
                page,
                page_size,
                query.as_deref(),
                order,
            );
            let _ = tx.send(WorkerResult::Envelopes(result));
        });
    }

    pub fn fetch_envelopes_threaded(
        &self,
        account: Option<String>,
        folder: String,
        query: Option<String>,
    ) {
        let tx = self.tx.clone();
        let service = self.service.clone();
        thread::spawn(move || {
            let result =
                service.list_envelopes_threaded(account.as_deref(), &folder, query.as_deref());
            let _ = tx.send(WorkerResult::Envelopes(result));
        });
    }

    /// Fetch unread count for a specific folder by querying for unseen envelopes.
    pub fn fetch_folder_unread(&self, account: Option<String>, folder: String) {
        let tx = self.tx.clone();
        let service = self.service.clone();
        let folder_name = folder.clone();
        thread::spawn(move || {
            let result = service.folder_unread(account.as_deref(), &folder);
            let _ = tx.send(WorkerResult::FolderUnread(folder_name, result));
        });
    }

    pub fn fetch_message(&self, account: Option<String>, folder: String, id: String) {
        let tx = self.tx.clone();
        let service = self.service.clone();
        thread::spawn(move || {
            let result = service.read_message_content(account.as_deref(), &folder, &id);
            let _ = tx.send(WorkerResult::Message(Box::new(result)));
        });
    }

    /// Resolve provider settings for an email address off the UI thread.
    pub fn discover_provider(&self, identifier: String) {
        let tx = self.tx.clone();
        thread::spawn(move || {
            let result = crate::mail::autoconfig::discover(&identifier);
            let _ = tx.send(WorkerResult::Discovered(result));
        });
    }

    /// Run `work` on a background thread and report it via `build`.
    pub(super) fn spawn<T: Send + 'static>(
        &self,
        work: impl FnOnce(&dyn MailService) -> T + Send + 'static,
        build: impl FnOnce(T) -> WorkerResult + Send + 'static,
    ) {
        let tx = self.tx.clone();
        let service = self.service.clone();
        thread::spawn(move || {
            let _ = tx.send(build(work(service.as_ref())));
        });
    }

    /// Run an action against the service on a background thread.
    pub(super) fn spawn_action<F>(&self, action: F)
    where
        F: FnOnce(&dyn MailService) -> MailResult<String> + Send + 'static,
    {
        self.spawn(action, WorkerResult::ActionDone);
    }

    pub fn delete_message(&self, account: Option<String>, folder: String, id: String) {
        self.spawn_action(move |service| {
            service
                .delete_message(account.as_deref(), &folder, &id)
                .map(|()| "Message deleted.".to_string())
        });
    }

    pub fn flag_add(&self, account: Option<String>, folder: String, id: String, flag: String) {
        self.spawn_action(move |service| {
            service
                .flag_add(account.as_deref(), &folder, &id, &flag)
                .map(|()| format!("Flag '{flag}' added."))
        });
    }

    pub fn flag_remove(&self, account: Option<String>, folder: String, id: String, flag: String) {
        self.spawn_action(move |service| {
            service
                .flag_remove(account.as_deref(), &folder, &id, &flag)
                .map(|()| format!("Flag '{flag}' removed."))
        });
    }

    pub fn move_message(
        &self,
        account: Option<String>,
        folder: String,
        target: String,
        id: String,
    ) {
        self.spawn_action(move |service| {
            service
                .move_message(account.as_deref(), &folder, &target, &id)
                .map(|()| format!("Moved to {target}."))
        });
    }

    /// Send a compiled template.
    pub fn send_template(&self, account: Option<String>, template: String, options: SendOptions) {
        let tx = self.tx.clone();
        let service = self.service.clone();
        thread::spawn(move || {
            let result = service
                .template_send(account.as_deref(), &template, &options)
                .map(|s| {
                    let s = s.trim();
                    if s.is_empty() {
                        "Message sent.".to_string()
                    } else {
                        s.to_string()
                    }
                });
            let _ = tx.send(WorkerResult::SendDone(result));
        });
    }

    /// Persist a compiled template as a draft, protected as `options` asks.
    pub fn save_draft(&self, account: Option<String>, template: String, options: SendOptions) {
        let tx = self.tx.clone();
        let service = self.service.clone();
        thread::spawn(move || {
            let result = service.save_draft(account.as_deref(), &template, &options);
            let _ = tx.send(WorkerResult::ActionDone(result));
        });
    }
}
