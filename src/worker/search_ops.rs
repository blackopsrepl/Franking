/*! Multi-folder search dispatched to the service. */

use crate::mail::search_merge::merge_results;

use super::dispatch::{Worker, WorkerResult};

impl Worker {
    /// Search every folder of every account and merge the results.
    ///
    /// Each account's folders come from its own listing, so a folder that only
    /// exists on one account is still searched.
    pub fn search_all_accounts(&self, query: String) {
        self.spawn(
            move |service| {
                let mut lists = Vec::new();
                let mut error = None;
                let accounts = service.list_accounts().unwrap_or_default();
                for account in &accounts {
                    let folders = service
                        .list_folders(Some(&account.name))
                        .unwrap_or_default();
                    for folder in folders {
                        match service.list_envelopes(
                            Some(&account.name),
                            &folder.name,
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
