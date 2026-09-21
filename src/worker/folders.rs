/*! Folder-level background operations. */

use std::thread;

use super::{Worker, WorkerResult};

impl Worker {
    pub fn mark_folder_seen(&self, account: Option<String>, folder: String) {
        let tx = self.tx.clone();
        let service = self.service.clone();
        thread::spawn(move || {
            let result = service
                .mark_folder_seen(account.as_deref(), &folder)
                .map(|()| format!("Marked {folder} as read."));
            let _ = tx.send(WorkerResult::ActionDone(result));
        });
    }

    pub fn sync_folder(&self, account: Option<String>, folder: String) {
        let tx = self.tx.clone();
        let service = self.service.clone();
        thread::spawn(move || {
            let result = service
                .sync_folder(account.as_deref(), &folder)
                .map(|envelopes| format!("Cached {} messages.", envelopes.len()));
            let _ = tx.send(WorkerResult::ActionDone(result));
        });
    }
}
