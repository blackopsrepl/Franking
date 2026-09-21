/*! Multi-folder search dispatched to the service. */

use crate::mail::search_merge::merge_results;

use super::dispatch::{Worker, WorkerResult};

impl Worker {
    /// Search every folder and merge the results.
    pub fn search_all_folders(&self, account: Option<String>, folders: Vec<String>, query: String) {
        self.spawn(
            move |service| {
                let mut lists = Vec::new();
                let mut error = None;
                for folder in &folders {
                    match service.list_envelopes(
                        account.as_deref(),
                        folder,
                        1,
                        50,
                        Some(query.as_str()),
                    ) {
                        Ok(envelopes) => lists.push(envelopes),
                        Err(failure) => {
                            if error.is_none() {
                                error = Some(failure);
                            }
                        }
                    }
                }
                if lists.is_empty() {
                    if let Some(failure) = error {
                        return Err(failure);
                    }
                }
                Ok(merge_results(lists))
            },
            WorkerResult::Envelopes,
        );
    }
}
