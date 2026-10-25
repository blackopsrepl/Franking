/*! Envelope listings apply only explicit future-mail rules. */

use super::{Worker, WorkerResult};
use std::thread;

impl Worker {
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
            let result = result.and_then(|rows| super::folder_rules::apply(service.as_ref(), rows));
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
            let result = result.and_then(|rows| super::folder_rules::apply(service.as_ref(), rows));
            let _ = tx.send(WorkerResult::Envelopes(result));
        });
    }
}
