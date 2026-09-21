/*! Saved searches: name the current query, and re-run stored ones. */

use crate::db::saved_searches::{self as store, SavedSearch};
use crate::keys::View;

use super::model::App;

/// State of the saved-search overlay.
#[derive(Default)]
pub struct SavedSearchesState {
    pub searches: Vec<SavedSearch>,
    pub index: usize,
    /// Name being typed when saving the active query.
    pub name_input: String,
    /// Whether the overlay is collecting a name.
    pub naming: bool,
    /// Name awaiting a second delete press.
    pub pending_delete: Option<String>,
}

impl SavedSearchesState {
    pub fn selected(&self) -> Option<&SavedSearch> {
        self.searches.get(self.index)
    }
}

impl App {
    /// Dispatch one saved-search action.
    pub(crate) fn handle_saved_search_action(&mut self, action: crate::keys::Action) {
        use crate::keys::Action;

        match action {
            Action::OpenSavedSearches => self.open_saved_searches(),
            Action::SaveSearch => self.begin_save_search(),
            Action::SaveSearchInput(c) => self.save_search_input(c),
            Action::SaveSearchBackspace => self.save_search_backspace(),
            Action::SaveSearchSubmit => self.submit_save_search(),
            Action::SaveSearchCancel => self.cancel_save_search(),
            Action::SavedSearchNext => self.saved_searches_next(),
            Action::SavedSearchPrev => self.saved_searches_prev(),
            Action::SavedSearchRun => self.run_saved_search(),
            Action::SavedSearchDelete => self.delete_saved_search(),
            Action::SavedSearchClose => self.close_saved_searches(),
            _ => {}
        }
    }

    /// Save the active query under a name, or say why it cannot be saved.
    pub(crate) fn begin_save_search(&mut self) {
        let Some(query) = self.active_query.clone() else {
            self.set_status("Run a search before saving it.");
            return;
        };
        if query.trim().is_empty() {
            self.set_status("Run a search before saving it.");
            return;
        }
        self.saved_searches.name_input = query.clone();
        self.saved_searches.naming = true;
        self.view = View::SaveSearch;
        self.set_status("Name this search, then press Enter.");
    }

    pub(crate) fn save_search_input(&mut self, c: char) {
        self.saved_searches.name_input.push(c);
    }

    pub(crate) fn save_search_backspace(&mut self) {
        self.saved_searches.name_input.pop();
    }

    /// Abandon the naming prompt.
    pub(crate) fn cancel_save_search(&mut self) {
        self.saved_searches.naming = false;
        self.saved_searches.name_input.clear();
        self.view = View::Search;
    }

    /// Store the active query under the typed name.
    pub(crate) fn submit_save_search(&mut self) {
        let Some(query) = self.active_query.clone() else {
            self.cancel_save_search();
            return;
        };
        let name = self.saved_searches.name_input.trim().to_string();
        if name.is_empty() {
            self.set_error("A name is required.");
            return;
        }
        let search = SavedSearch {
            name: name.clone(),
            query,
            scope: self.search_scope,
        };
        let Some(conn) = self.db.as_ref() else {
            self.set_error("Local database is unavailable.");
            return;
        };
        match store::save(conn, &search) {
            Ok(()) => {
                self.saved_searches.naming = false;
                self.saved_searches.name_input.clear();
                self.view = View::Search;
                self.set_status(&format!("Saved the search as {name}."));
            }
            Err(error) => self.set_error(&format!("Could not save the search: {error}")),
        }
    }

    /// Open the saved-search overlay.
    pub(crate) fn open_saved_searches(&mut self) {
        self.saved_searches.naming = false;
        self.saved_searches.pending_delete = None;
        self.reload_saved_searches();
        self.view = View::SavedSearches;
    }

    fn reload_saved_searches(&mut self) {
        let searches = self
            .db
            .as_ref()
            .and_then(|conn| store::list(conn).ok())
            .unwrap_or_default();
        self.saved_searches.searches = searches;
        self.saved_searches.index = self
            .saved_searches
            .index
            .min(self.saved_searches.searches.len().saturating_sub(1));
    }

    pub(crate) fn saved_searches_next(&mut self) {
        if !self.saved_searches.searches.is_empty() {
            self.saved_searches.index =
                (self.saved_searches.index + 1).min(self.saved_searches.searches.len() - 1);
        }
    }

    pub(crate) fn saved_searches_prev(&mut self) {
        self.saved_searches.index = self.saved_searches.index.saturating_sub(1);
    }

    /// Run the highlighted saved search.
    pub(crate) fn run_saved_search(&mut self) {
        let Some(search) = self.saved_searches.selected().cloned() else {
            return;
        };
        self.search_query = search.query.clone();
        self.search_scope = search.scope;
        self.view = View::EnvelopeList;
        self.submit_search();
    }

    /// Delete the highlighted saved search after a second press.
    pub(crate) fn delete_saved_search(&mut self) {
        let Some(search) = self.saved_searches.selected() else {
            return;
        };
        let name = search.name.clone();
        if self.saved_searches.pending_delete.as_deref() != Some(name.as_str()) {
            self.saved_searches.pending_delete = Some(name);
            self.set_status("Press d again to delete this saved search.");
            return;
        }
        self.saved_searches.pending_delete = None;
        let Some(conn) = self.db.as_ref() else {
            self.set_error("Local database is unavailable.");
            return;
        };
        match store::delete(conn, &name) {
            Ok(true) => {
                self.reload_saved_searches();
                self.set_status(&format!("Deleted the saved search {name}."));
            }
            Ok(false) => self.set_status("That saved search is gone."),
            Err(error) => self.set_error(&format!("Could not delete the search: {error}")),
        }
    }

    /// Close the overlay.
    pub(crate) fn close_saved_searches(&mut self) {
        self.saved_searches.pending_delete = None;
        self.view = View::EnvelopeList;
    }
}
