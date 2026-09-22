/*! Local-first search results.
The cache answers a query immediately; the server result replaces it when it
arrives, which keeps search usable offline and instant online. */

use super::model::App;

/// How many cached matches to show while the server is still answering.
const CACHED_MATCH_LIMIT: usize = 200;

impl App {
    /// Show locally cached matches for `query`, so a search answers at once.
    ///
    /// The server result replaces this list when it arrives; this is what makes
    /// a search usable while offline and instant while online.
    pub(crate) fn show_cached_matches(&mut self, query: &str) {
        let Some(conn) = self.db.as_ref() else {
            return;
        };
        let scope = self.search_scope;
        let account = if scope == crate::mail::search_scope::SearchScope::Accounts {
            None
        } else {
            self.acct_owned().as_deref().map(str::to_string)
        };

        let stored = match crate::mail::store::search_messages(
            conn,
            account.as_deref(),
            query,
            CACHED_MATCH_LIMIT,
        ) {
            Ok(messages) => messages,
            Err(_) => return,
        };
        let mut envelopes: Vec<crate::mail::types::Envelope> = stored
            .iter()
            .map(crate::mail::store::StoredMessage::to_envelope)
            .collect();
        if scope == crate::mail::search_scope::SearchScope::Folder {
            let folder = self.current_folder.clone();
            envelopes.retain(|envelope| envelope.folder.as_deref() == Some(folder.as_str()));
        }
        if envelopes.is_empty() {
            return;
        }

        let count = envelopes.len();
        self.envelopes = envelopes;
        self.envelope_state.select(Some(0));
        self.set_status(&format!(
            "{count} cached match(es); searching the server..."
        ));
    }
}
