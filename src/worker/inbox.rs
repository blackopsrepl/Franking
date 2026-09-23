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
            let accounts = match service.list_accounts() {
                Ok(accounts) => accounts,
                Err(error) => {
                    let _ = tx.send(WorkerResult::Envelopes(Err(error)));
                    return;
                }
            };
            for account in accounts {
                match service.list_envelopes(Some(&account.name), &folder, 1, 200, None) {
                    Ok(envelopes) => merged.extend(envelopes),
                    Err(mut error) => {
                        error.detail = format!("{}: {}", account.name, error.detail);
                        let _ = tx.send(WorkerResult::Envelopes(Err(error)));
                        return;
                    }
                }
            }
            merged.sort_by(|left, right| right.date.cmp(&left.date));
            let _ = tx.send(WorkerResult::Envelopes(Ok(merged)));
        });
    }
}
