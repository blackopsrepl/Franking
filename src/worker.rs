use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::mail::types::*;
use crate::mail::{
    default_mail_service, IdleOutcome, MailError, MailErrorKind, MailService, MessageDocument,
};

const IDLE_TIMEOUT: Duration = Duration::from_secs(5);
const WATCH_BACKOFF: Duration = Duration::from_secs(5);

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
}

/// A running IDLE watcher and the flag used to stop it.
struct Watcher {
    stop: Arc<AtomicBool>,
    _handle: JoinHandle<()>,
}

/// Lightweight handle for dispatching work to background threads.
/// Results are collected via try_recv() in the main loop.
pub struct Worker {
    tx: mpsc::Sender<WorkerResult>,
    rx: mpsc::Receiver<WorkerResult>,
    service: Arc<dyn MailService>,
    watcher: Mutex<Option<Watcher>>,
}

impl Default for Worker {
    fn default() -> Self {
        Self::new()
    }
}

impl Worker {
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

    /// Watch a folder for changes using IMAP IDLE, replacing any prior watcher.
    /// Backends without push support stop immediately.
    pub fn start_watching(&self, account: Option<String>, folder: String) {
        if let Ok(mut guard) = self.watcher.lock() {
            if let Some(previous) = guard.take() {
                previous.stop.store(true, Ordering::Relaxed);
            }
        }

        let stop = Arc::new(AtomicBool::new(false));
        let tx = self.tx.clone();
        let service = self.service.clone();
        let thread_stop = Arc::clone(&stop);
        let thread_account = account;
        let thread_folder = folder;

        let handle = thread::spawn(move || {
            while !thread_stop.load(Ordering::Relaxed) {
                match service.idle_watch(thread_account.as_deref(), &thread_folder, IDLE_TIMEOUT) {
                    Ok(IdleOutcome::MailboxChanged) => {
                        if tx
                            .send(WorkerResult::MailboxChanged(
                                thread_account.clone(),
                                thread_folder.clone(),
                            ))
                            .is_err()
                        {
                            break;
                        }
                    }
                    Ok(IdleOutcome::TimedOut) => {}
                    Err(error) if error.kind == MailErrorKind::UnsupportedFeature => break,
                    Err(_) => thread::sleep(WATCH_BACKOFF),
                }
            }
        });

        if let Ok(mut guard) = self.watcher.lock() {
            *guard = Some(Watcher {
                stop,
                _handle: handle,
            });
        }
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

    pub fn fetch_folders(&self, account: Option<String>) {
        let tx = self.tx.clone();
        let service = self.service.clone();
        thread::spawn(move || {
            let result = service.list_folders(account.as_deref());
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
    ) {
        let tx = self.tx.clone();
        let service = self.service.clone();
        thread::spawn(move || {
            let result = service.list_envelopes(
                account.as_deref(),
                &folder,
                page,
                page_size,
                query.as_deref(),
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
            // Query for unseen envelopes with page-size 1 to get a count.
            // We use the envelope list with "not flag seen" filter.
            let result = service
                .list_envelopes(account.as_deref(), &folder, 1, 200, Some("not flag seen"))
                .map(|envs| envs.len());
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

    pub fn delete_message(&self, account: Option<String>, folder: String, id: String) {
        let tx = self.tx.clone();
        let service = self.service.clone();
        thread::spawn(move || {
            let result = service
                .delete_message(account.as_deref(), &folder, &id)
                .map(|()| "Message deleted.".to_string());
            let _ = tx.send(WorkerResult::ActionDone(result));
        });
    }

    pub fn flag_add(&self, account: Option<String>, folder: String, id: String, flag: String) {
        let tx = self.tx.clone();
        let service = self.service.clone();
        thread::spawn(move || {
            let result = service
                .flag_add(account.as_deref(), &folder, &id, &flag)
                .map(|()| format!("Flag '{flag}' added."));
            let _ = tx.send(WorkerResult::ActionDone(result));
        });
    }

    pub fn flag_remove(&self, account: Option<String>, folder: String, id: String, flag: String) {
        let tx = self.tx.clone();
        let service = self.service.clone();
        thread::spawn(move || {
            let result = service
                .flag_remove(account.as_deref(), &folder, &id, &flag)
                .map(|()| format!("Flag '{flag}' removed."));
            let _ = tx.send(WorkerResult::ActionDone(result));
        });
    }

    pub fn move_message(
        &self,
        account: Option<String>,
        folder: String,
        target: String,
        id: String,
    ) {
        let tx = self.tx.clone();
        let service = self.service.clone();
        thread::spawn(move || {
            let result = service
                .move_message(account.as_deref(), &folder, &target, &id)
                .map(|()| format!("Moved to {target}."));
            let _ = tx.send(WorkerResult::ActionDone(result));
        });
    }

    pub fn download_attachments(&self, account: Option<String>, folder: String, id: String) {
        let tx = self.tx.clone();
        let service = self.service.clone();
        thread::spawn(move || {
            let result = service
                .download_attachments(account.as_deref(), &folder, &id)
                .map(|s| format!("Attachments: {}", s.trim()));
            let _ = tx.send(WorkerResult::ActionDone(result));
        });
    }

    /// Fetch a compose template (new message).
    pub fn fetch_template_write(&self, account: Option<String>) {
        let tx = self.tx.clone();
        let service = self.service.clone();
        thread::spawn(move || {
            let result = service.template_write(account.as_deref());
            let _ = tx.send(WorkerResult::Template(result));
        });
    }

    /// Fetch a reply template.
    pub fn fetch_template_reply(
        &self,
        account: Option<String>,
        folder: String,
        id: String,
        all: bool,
    ) {
        let tx = self.tx.clone();
        let service = self.service.clone();
        thread::spawn(move || {
            let result = service.template_reply(account.as_deref(), &folder, &id, all);
            let _ = tx.send(WorkerResult::Template(result));
        });
    }

    /// Fetch a forward template.
    pub fn fetch_template_forward(&self, account: Option<String>, folder: String, id: String) {
        let tx = self.tx.clone();
        let service = self.service.clone();
        thread::spawn(move || {
            let result = service.template_forward(account.as_deref(), &folder, &id);
            let _ = tx.send(WorkerResult::Template(result));
        });
    }

    /// Send a compiled template.
    pub fn send_template(&self, account: Option<String>, template: String) {
        let tx = self.tx.clone();
        let service = self.service.clone();
        thread::spawn(move || {
            let result = service
                .template_send(account.as_deref(), &template)
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

    /// Persist a compiled template as a draft.
    pub fn save_draft(&self, account: Option<String>, template: String) {
        let tx = self.tx.clone();
        let service = self.service.clone();
        thread::spawn(move || {
            let result = service.save_draft(account.as_deref(), &template);
            let _ = tx.send(WorkerResult::ActionDone(result));
        });
    }
}
