/*! Dispatch for outgoing mail: sending, drafts, and calendar hand-off. */

use super::dispatch::{Worker, WorkerResult};

impl Worker {
    /// Hand an invitation to Planner123 and report what it did.
    ///
    /// The CLI runs synchronously: an import is a short, local operation, and
    /// the caller wants its report rather than a handle to poll.
    pub fn add_to_planner(&self, ics: Vec<u8>, client: crate::mail::planner123::Planner123) {
        self.spawn(
            move |_service| {
                client
                    .import_invitation(&ics)
                    .map(|report| report.summary())
            },
            WorkerResult::ActionDone,
        );
    }
}
