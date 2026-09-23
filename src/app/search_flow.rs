/*! The search prompt, scope, and submission. */

use crate::keys::View;

use super::model::App;

impl App {
    /// Pre-selects the default identity if one exists.
    pub(crate) fn enter_search(&mut self) {
        self.search_query.clear();
        self.view = View::Search;
    }

    /// Delete the last character of the current search query.
    pub(crate) fn search_backspace(&mut self) {
        self.search_query.pop();
    }

    /// Widen the next search: this folder, all folders, all accounts.
    pub(crate) fn toggle_search_scope(&mut self) {
        self.search_scope = self.search_scope.next();
        self.set_status(&format!("Search scope: {}.", self.search_scope.label()));
    }

    pub(crate) fn submit_search(&mut self) {
        self.triage_lane = None;
        self.followup_lane = None;
        let query = self.search_query.clone();
        self.active_query = if query.is_empty() { None } else { Some(query) };
        self.page = 1;
        let scope = self.search_scope;
        let query = self.active_query.clone();
        self.view = View::EnvelopeList;

        // Local results first: the cache answers immediately, and the server
        // result replaces them when it arrives.
        if let Some(query) = query.as_deref() {
            self.show_cached_matches(query);
        }

        if scope == crate::mail::search_scope::SearchScope::Accounts {
            if let Some(query) = query {
                self.loading = true;
                self.set_status("Searching every account...");
                self.worker.search_all_accounts(query);
                return;
            }
        }

        if scope == crate::mail::search_scope::SearchScope::Folders {
            if let Some(query) = query {
                let folders: Vec<String> = self
                    .folders
                    .iter()
                    .filter(|folder| folder.name != super::model::UNIFIED_INBOX)
                    .map(|folder| folder.name.clone())
                    .collect();
                self.loading = true;
                self.set_status(&format!("Searching {} folders...", folders.len()));
                self.worker
                    .search_all_folders(self.acct_owned(), folders, query);
                return;
            }
        }
        self.load_envelopes();
    }

    pub(crate) fn cancel_search(&mut self) {
        self.view = View::EnvelopeList;
    }
}
