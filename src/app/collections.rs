/*! Named collections: group related conversations without merging them. */

use crate::db::collections::{self as store, Collection};
use crate::keys::View;

use super::model::App;

/// Which collection-name prompt is open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameKind {
    New,
    Rename(i64),
}

/// State of the collection board.
#[derive(Default)]
pub struct CollectionsState {
    pub account: String,
    pub items: Vec<Collection>,
    pub index: usize,
    /// Anchors of the conversation the board was opened on.
    pub target: Vec<String>,
    pub input: String,
    pub naming: Option<NameKind>,
    pub pending_delete: Option<i64>,
}

impl App {
    pub(crate) fn open_collections(&mut self) {
        self.collections.account = self.acct_owned().unwrap_or_default();
        self.collections.index = 0;
        self.collections.pending_delete = None;
        self.collections.target = self
            .selected_envelope()
            .and_then(|envelope| self.conversation_anchors.get(&envelope.id).cloned())
            .unwrap_or_default();
        self.reload_collections();
        self.view = View::CollectionBoard;
    }

    fn reload_collections(&mut self) {
        let account = self.collections.account.clone();
        self.collections.items = self
            .db
            .as_ref()
            .and_then(|conn| store::list(conn, &account).ok())
            .unwrap_or_default();
        self.collections.index = self
            .collections
            .index
            .min(self.collections.items.len().saturating_sub(1));
    }

    pub(crate) fn collections_next(&mut self) {
        if !self.collections.items.is_empty() {
            self.collections.index =
                (self.collections.index + 1).min(self.collections.items.len() - 1);
        }
    }

    pub(crate) fn collections_prev(&mut self) {
        self.collections.index = self.collections.index.saturating_sub(1);
    }

    pub(crate) fn close_collections(&mut self) {
        self.collections.pending_delete = None;
        self.view = View::EnvelopeList;
    }

    /// Add or remove the conversation from the highlighted collection.
    pub(crate) fn collections_toggle(&mut self) {
        let Some(collection) = self.collections.items.get(self.collections.index).cloned() else {
            return;
        };
        if self.collections.target.is_empty() {
            self.set_status("No conversation to file.");
            return;
        }
        let (account, anchors) = (
            self.collections.account.clone(),
            self.collections.target.clone(),
        );
        let Some(conn) = self.db.as_ref() else {
            self.set_error("Local database is unavailable.");
            return;
        };
        match store::toggle(conn, &account, collection.id, &anchors) {
            Ok(added) => self.set_status(if added {
                "Added to the collection."
            } else {
                "Removed from the collection."
            }),
            Err(error) => self.set_error(&format!("Could not update the collection: {error}")),
        }
    }

    /// Focus the highlighted collection as a list filter, or clear it.
    pub(crate) fn collections_filter(&mut self) {
        let Some(collection) = self.collections.items.get(self.collections.index) else {
            return;
        };
        let name = collection.name.clone();
        if self.collection_filter.as_deref() == Some(name.as_str()) {
            self.collection_filter = None;
            self.set_status("Collection filter cleared.");
        } else {
            self.collection_filter = Some(name.clone());
            self.set_status(&format!("Showing collection {name}."));
        }
        self.close_collections();
        self.load_envelopes();
    }

    pub(crate) fn collections_begin_new(&mut self) {
        self.collections.input.clear();
        self.collections.naming = Some(NameKind::New);
        self.view = View::CollectionName;
    }

    pub(crate) fn collections_begin_rename(&mut self) {
        let Some(collection) = self.collections.items.get(self.collections.index) else {
            return;
        };
        self.collections.input = collection.name.clone();
        self.collections.naming = Some(NameKind::Rename(collection.id));
        self.view = View::CollectionName;
    }

    pub(crate) fn collection_name_input(&mut self, c: char) {
        self.collections.input.push(c);
    }

    pub(crate) fn collection_name_backspace(&mut self) {
        self.collections.input.pop();
    }

    pub(crate) fn cancel_collection_name(&mut self) {
        self.collections.naming = None;
        self.collections.input.clear();
        self.view = View::CollectionBoard;
    }

    pub(crate) fn submit_collection_name(&mut self) {
        let name = self.collections.input.trim().to_string();
        if name.is_empty() {
            self.set_error("A collection name is required.");
            return;
        }
        let kind = self.collections.naming;
        let account = self.collections.account.clone();
        let Some(conn) = self.db.as_ref() else {
            self.set_error("Local database is unavailable.");
            return;
        };
        let result = match kind {
            Some(NameKind::Rename(id)) => store::rename(conn, id, &name),
            _ => store::upsert(conn, &account, &name).map(|_| ()),
        };
        match result {
            Ok(()) => {
                self.collections.naming = None;
                self.collections.input.clear();
                self.reload_collections();
                self.view = View::CollectionBoard;
                self.set_status(&format!("Collection {name} saved."));
            }
            Err(error) => self.set_error(&format!("Could not save the collection: {error}")),
        }
    }

    pub(crate) fn collections_delete(&mut self) {
        let Some(collection) = self.collections.items.get(self.collections.index) else {
            return;
        };
        let id = collection.id;
        if self.collections.pending_delete != Some(id) {
            self.collections.pending_delete = Some(id);
            self.set_status("Press d again to delete this collection.");
            return;
        }
        self.collections.pending_delete = None;
        let Some(conn) = self.db.as_ref() else {
            self.set_error("Local database is unavailable.");
            return;
        };
        match store::delete(conn, id) {
            Ok(_) => {
                self.reload_collections();
                self.set_status("Collection deleted.");
            }
            Err(error) => self.set_error(&format!("Could not delete the collection: {error}")),
        }
    }
}

impl App {
    /// Record each loaded conversation's collections and apply the active filter.
    pub(crate) fn load_collections_and_filter(&mut self, anchors_by_row: &[Vec<String>]) {
        self.collection_of_anchor.clear();
        let Some(conn) = self.db.as_ref() else {
            return;
        };
        let mut by_account: std::collections::HashMap<
            String,
            std::collections::HashMap<String, Vec<String>>,
        > = std::collections::HashMap::new();
        for (index, envelope) in self.envelopes.iter().enumerate() {
            let Some(account) = envelope
                .account
                .clone()
                .or_else(|| self.account_name.clone())
                .filter(|account| !account.is_empty())
            else {
                continue;
            };
            let membership = by_account.entry(account.clone()).or_insert_with(|| {
                store::membership_for_account(conn, &account).unwrap_or_default()
            });
            for anchor in &anchors_by_row[index] {
                if let Some(names) = membership.get(anchor) {
                    self.collection_of_anchor
                        .entry(anchor.clone())
                        .or_default()
                        .extend(names.iter().cloned());
                }
            }
        }
        if let Some(filter) = self.collection_filter.clone() {
            let anchors = &self.conversation_anchors;
            let of_anchor = &self.collection_of_anchor;
            self.envelopes.retain(|envelope| {
                anchors.get(&envelope.id).is_some_and(|list| {
                    list.iter().any(|anchor| {
                        of_anchor
                            .get(anchor)
                            .is_some_and(|names| names.iter().any(|name| name == &filter))
                    })
                })
            });
        }
    }
}
