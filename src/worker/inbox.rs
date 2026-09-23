/*! Unified inbox aggregation. */

use std::thread;

use super::{Worker, WorkerResult};

/// Largest per-account window a merged view will fetch, so paging cannot grow
/// without bound.
pub(crate) const MAX_PER_ACCOUNT: usize = 2000;

impl Worker {
    /// List every account's inbox up to `per_account` messages and merge by
    /// date. The caller grows `per_account` as the user pages.
    pub fn fetch_all_inboxes(&self, folder: String, per_account: usize) {
        let tx = self.tx.clone();
        let service = self.service.clone();
        let per_account = per_account.clamp(1, MAX_PER_ACCOUNT);
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
                match service.list_envelopes(Some(&account.name), &folder, 1, per_account, None) {
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
