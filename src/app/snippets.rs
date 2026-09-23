/*! Reusable compose snippets: browse, insert, save the current body, delete. */

use crate::db::snippets::{self as store, Snippet};
use crate::keys::View;

use super::model::App;

/// State of the snippet picker.
#[derive(Default)]
pub struct SnippetsState {
    pub items: Vec<Snippet>,
    pub index: usize,
    /// Snippet name awaiting a second delete press.
    pub pending_delete: Option<String>,
    /// Name being typed when saving the current compose body.
    pub naming: bool,
    pub name_input: String,
}

impl App {
    pub(crate) fn open_snippets(&mut self) {
        self.snippets.pending_delete = None;
        self.snippets.naming = false;
        self.reload_snippets();
        self.view = View::Snippets;
    }

    fn reload_snippets(&mut self) {
        self.snippets.items = self
            .db
            .as_ref()
            .and_then(|conn| store::list(conn).ok())
            .unwrap_or_default();
        self.snippets.index = self
            .snippets
            .index
            .min(self.snippets.items.len().saturating_sub(1));
    }

    pub(crate) fn snippets_next(&mut self) {
        if !self.snippets.items.is_empty() {
            self.snippets.index = (self.snippets.index + 1).min(self.snippets.items.len() - 1);
        }
    }

    pub(crate) fn snippets_prev(&mut self) {
        self.snippets.index = self.snippets.index.saturating_sub(1);
    }

    /// Insert the highlighted snippet into the composition at the cursor.
    pub(crate) fn insert_snippet(&mut self) {
        let Some(snippet) = self.snippets.items.get(self.snippets.index).cloned() else {
            return;
        };
        if let Some(cs) = self.compose_state.as_mut() {
            cs.body.insert_str(&snippet.body);
            cs.dirty = true;
        } else {
            let mut cs = crate::compose::ComposeState::new(
                crate::compose::ComposeMode::New,
                self.acct_owned(),
            );
            cs.body = crate::compose_editor::ComposeEditor::from_text(&snippet.body);
            self.load_identities_into(&mut cs);
            self.compose_state = Some(cs);
        }
        self.view = View::Compose;
        self.set_status(&format!("Inserted snippet {}.", snippet.name));
    }

    pub(crate) fn snippets_close(&mut self) {
        self.snippets.pending_delete = None;
        self.view = View::Compose;
        if self.compose_state.is_none() {
            self.view = View::EnvelopeList;
        }
    }

    /// Start naming the current compose body as a snippet.
    pub(crate) fn begin_save_snippet(&mut self) {
        let body = self
            .compose_state
            .as_ref()
            .map(|cs| cs.body.text())
            .unwrap_or_default();
        if body.trim().is_empty() {
            self.set_status("Write something in the body first.");
            return;
        }
        self.snippets.naming = true;
        self.snippets.name_input = body
            .lines()
            .next()
            .unwrap_or_default()
            .trim()
            .chars()
            .take(40)
            .collect();
        self.view = View::SnippetName;
    }

    pub(crate) fn snippet_name_input(&mut self, c: char) {
        self.snippets.name_input.push(c);
    }

    pub(crate) fn snippet_name_backspace(&mut self) {
        self.snippets.name_input.pop();
    }

    pub(crate) fn cancel_snippet_name(&mut self) {
        self.snippets.naming = false;
        self.snippets.name_input.clear();
        self.view = View::Compose;
    }

    pub(crate) fn submit_snippet_name(&mut self) {
        let name = self.snippets.name_input.trim().to_string();
        if name.is_empty() {
            self.set_error("A snippet name is required.");
            return;
        }
        let body = self
            .compose_state
            .as_ref()
            .map(|cs| cs.body.text())
            .unwrap_or_default();
        let Some(conn) = self.db.as_ref() else {
            self.set_error("Local database is unavailable.");
            return;
        };
        match store::save(
            conn,
            &Snippet {
                name: name.clone(),
                body,
            },
        ) {
            Ok(()) => {
                self.snippets.naming = false;
                self.snippets.name_input.clear();
                self.reload_snippets();
                self.view = View::Snippets;
                self.set_status(&format!("Saved snippet {name}."));
            }
            Err(error) => self.set_error(&format!("Could not save the snippet: {error}")),
        }
    }

    pub(crate) fn delete_snippet(&mut self) {
        let Some(snippet) = self.snippets.items.get(self.snippets.index) else {
            return;
        };
        let name = snippet.name.clone();
        if self.snippets.pending_delete.as_deref() != Some(name.as_str()) {
            self.snippets.pending_delete = Some(name);
            self.set_status("Press d again to delete this snippet.");
            return;
        }
        self.snippets.pending_delete = None;
        let Some(conn) = self.db.as_ref() else {
            self.set_error("Local database is unavailable.");
            return;
        };
        match store::delete(conn, &name) {
            Ok(_) => {
                self.reload_snippets();
                self.set_status(&format!("Deleted snippet {name}."));
            }
            Err(error) => self.set_error(&format!("Could not delete the snippet: {error}")),
        }
    }
}
