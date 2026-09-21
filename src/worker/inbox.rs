/*! Unified inbox aggregation. */

use std::thread;

use super::{Worker, WorkerResult};

impl Worker {
    /// List every account's inbox and merge by date.
    pub fn fetch_all_inboxes(&self, folder: String) {
        let tx = self.tx.clone();
        let service = self.service.clone();
        thread::spawn(move || {
            let mut merged = Vec::new();
            if let Ok(accounts) = service.list_accounts() {
                for account in accounts {
                    if let Ok(envelopes) =
                        service.list_envelopes(Some(&account.name), &folder, 1, 200, None)
                    {
                        merged.extend(envelopes);
                    }
                }
            }
            merged.sort_by(|left, right| right.date.cmp(&left.date));
            let _ = tx.send(WorkerResult::Envelopes(Ok(merged)));
        });
    }
}
