/*! Envelope listings: server truth, then sender-rule routing when the App
passed a snapshot of active rules. */

use super::folder_rules::{deliver_listing, RoutingSnapshot};
use super::Worker;
use std::thread;

impl Worker {
    /// `(page, page_size)` selects the window.
    pub fn fetch_envelopes(
        &self,
        account: Option<String>,
        folder: String,
        (page, page_size): (usize, usize),
        query: Option<String>,
        order: crate::mail::sort::SortOrder,
        rules: Option<RoutingSnapshot>,
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
            deliver_listing(&tx, service.as_ref(), rules.as_ref(), result);
        });
    }

    pub fn fetch_envelopes_threaded(
        &self,
        account: Option<String>,
        folder: String,
        query: Option<String>,
        rules: Option<RoutingSnapshot>,
    ) {
        let tx = self.tx.clone();
        let service = self.service.clone();
        thread::spawn(move || {
            let result =
                service.list_envelopes_threaded(account.as_deref(), &folder, query.as_deref());
            deliver_listing(&tx, service.as_ref(), rules.as_ref(), result);
        });
    }
}
