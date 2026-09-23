/*! IMAP IDLE watching.
Runs a dedicated thread that waits for mailbox changes and forwards them to the
main loop as `WorkerResult::MailboxChanged`. */

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::mail::{IdleOutcome, MailErrorKind};

use super::{Worker, WorkerResult};

const IDLE_TIMEOUT: Duration = Duration::from_secs(5);
const WATCH_BACKOFF: Duration = Duration::from_secs(5);

/// A running IDLE watcher and the flag used to stop it.
pub(super) struct Watcher {
    pub(super) stop: Arc<AtomicBool>,
    pub(super) _handle: JoinHandle<()>,
}

impl Worker {
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
                        let newest = service
                            .list_envelopes(thread_account.as_deref(), &thread_folder, 1, 1, None)
                            .ok()
                            .and_then(|envelopes| envelopes.into_iter().next());
                        if tx
                            .send(WorkerResult::MailboxChanged(
                                thread_account.clone(),
                                thread_folder.clone(),
                                newest,
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
}
