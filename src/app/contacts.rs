/*! Contact browser and edit form handling. */

use crate::contact_edit::ContactEditState;
use crate::keys::{self, View};

use super::model::App;

impl App {
    pub(crate) fn open_contacts(&mut self) {
        // Load contacts from DB if available.
        if let Some(ref conn) = self.db {
            match crate::contacts::list(conn, self.contact_tag_filter.as_deref()) {
                Ok(contacts) => {
                    self.contacts = contacts;
                    self.contact_index = if self.contacts.is_empty() {
                        None
                    } else {
                        Some(0)
                    };
                }
                Err(e) => {
                    self.set_error(&format!("contacts: {e}"));
                    return;
                }
            }
        }
        self.previous_view = Some(self.view);
        self.view = View::Contacts;
    }

    pub(crate) fn contact_search_start(&mut self) {
        self.contact_search.clear();
        self.contact_search_active = true;
        self.view = keys::View::ContactSearch;
    }

    pub(crate) fn contact_search_input(&mut self, c: char) {
        self.contact_search.push(c);
        self.refresh_contact_search();
    }

    pub(crate) fn contact_search_backspace(&mut self) {
        self.contact_search.pop();
        self.refresh_contact_search();
    }

    pub(crate) fn contact_search_cancel(&mut self) {
        self.contact_search_active = false;
        self.view = keys::View::Contacts;
        // If search is cleared, reload full list; otherwise keep current results.
        if self.contact_search.is_empty() {
            if let Some(ref conn) = self.db {
                if let Ok(contacts) =
                    crate::contacts::list(conn, self.contact_tag_filter.as_deref())
                {
                    self.contacts = contacts;
                    self.contact_index = if self.contacts.is_empty() {
                        None
                    } else {
                        Some(0)
                    };
                }
            }
        }
    }

    /// Run a live contact search and update `self.contacts`.
    pub(crate) fn refresh_contact_search(&mut self) {
        if self.contact_search.is_empty() {
            // Show all contacts when query is empty
            if let Some(ref conn) = self.db {
                if let Ok(contacts) =
                    crate::contacts::list(conn, self.contact_tag_filter.as_deref())
                {
                    self.contacts = contacts;
                    self.contact_index = if self.contacts.is_empty() {
                        None
                    } else {
                        Some(0)
                    };
                }
            }
            return;
        }
        if let Some(ref conn) = self.db {
            match crate::contacts::search(conn, &self.contact_search.clone(), 50) {
                Ok(contacts) => {
                    self.contacts = contacts;
                    self.contact_index = if self.contacts.is_empty() {
                        None
                    } else {
                        Some(0)
                    };
                }
                Err(e) => self.set_error(&format!("search: {e}")),
            }
        }
    }

    pub(crate) fn contact_new(&mut self) {
        self.contact_edit_state = Some(ContactEditState::new());
        self.previous_view = Some(self.view);
        self.view = keys::View::ContactEdit;
    }

    pub(crate) fn contact_edit_selected(&mut self) {
        let contact = self
            .contact_index
            .and_then(|i| self.contacts.get(i))
            .cloned();
        if let Some(c) = contact {
            self.contact_edit_state = Some(ContactEditState::from_contact(&c));
            self.previous_view = Some(self.view);
            self.view = keys::View::ContactEdit;
        }
    }

    pub(crate) fn contact_edit_field_next(&mut self) {
        if let Some(ref mut s) = self.contact_edit_state {
            s.focused = s.focused.next();
        }
    }

    pub(crate) fn contact_edit_field_prev(&mut self) {
        if let Some(ref mut s) = self.contact_edit_state {
            s.focused = s.focused.prev();
        }
    }

    pub(crate) fn contact_edit_input(&mut self, c: char) {
        if let Some(ref mut s) = self.contact_edit_state {
            if let Some(field) = s.focused_field_mut() {
                field.push(c);
            }
            s.error = None;
        }
    }

    pub(crate) fn contact_edit_backspace(&mut self) {
        if let Some(ref mut s) = self.contact_edit_state {
            if let Some(field) = s.focused_field_mut() {
                field.pop();
            }
        }
    }

    /// Enter key in contact edit: activate action buttons or advance field.
    pub(crate) fn contact_edit_activate(&mut self) {
        let focused = self.contact_edit_state.as_ref().map(|s| s.focused);
        match focused {
            Some(crate::contact_edit::ContactField::Save) => self.contact_edit_save(),
            Some(crate::contact_edit::ContactField::Cancel) => self.contact_edit_cancel(),
            Some(_) => self.contact_edit_field_next(),
            None => {}
        }
    }

    pub(crate) fn contact_edit_save(&mut self) {
        let Some(ref s) = self.contact_edit_state else {
            return;
        };
        match s.to_contact() {
            Ok(contact) => {
                if let Some(ref conn) = self.db {
                    // Insert or update the contact record.
                    let save_result = if contact.id == 0 {
                        crate::contacts::add(conn, &contact)
                    } else {
                        crate::contacts::update(conn, &contact).map(|_| contact.id)
                    };
                    match save_result {
                        Ok(contact_id) => {
                            // Sync tags: remove all existing tags then re-add.
                            let _ = conn.execute(
                                "DELETE FROM contact_tags WHERE contact_id = ?1",
                                [contact_id],
                            );
                            for tag in &contact.tags {
                                let _ = crate::contacts::add_tag(conn, contact_id, tag);
                            }
                            self.contact_edit_state = None;
                            let prev = self.previous_view.unwrap_or(keys::View::Contacts);
                            self.view = prev;
                            self.previous_view = None;
                            self.set_status("Contact saved.");
                            // Reload contact list
                            if let Some(ref conn) = self.db {
                                if let Ok(contacts) =
                                    crate::contacts::list(conn, self.contact_tag_filter.as_deref())
                                {
                                    self.contacts = contacts;
                                    self.contact_index = if self.contacts.is_empty() {
                                        None
                                    } else {
                                        Some(0)
                                    };
                                }
                            }
                        }
                        Err(e) => {
                            if let Some(ref mut s) = self.contact_edit_state {
                                s.error = Some(format!("Save failed: {e}"));
                            }
                        }
                    }
                }
            }
            Err(msg) => {
                if let Some(ref mut s) = self.contact_edit_state {
                    s.error = Some(msg);
                }
            }
        }
    }

    pub(crate) fn contact_edit_cancel(&mut self) {
        self.contact_edit_state = None;
        let prev = self.previous_view.unwrap_or(keys::View::Contacts);
        self.view = prev;
        self.previous_view = None;
    }

    pub(crate) fn contact_delete(&mut self) {
        let Some(idx) = self.contact_index else {
            return;
        };
        let Some(contact) = self.contacts.get(idx) else {
            return;
        };
        let id = contact.id;

        // Deleting a contact loses data with no undo, so it takes a second
        // press, as deleting a message, a folder, or a queued send does.
        if self.contact_pending_delete != Some(id) {
            self.contact_pending_delete = Some(id);
            self.set_status("Press d again to delete this contact.");
            return;
        }
        self.contact_pending_delete = None;

        if let Some(ref conn) = self.db {
            match crate::contacts::delete(conn, id) {
                Ok(()) => {
                    self.contacts.remove(idx);
                    if !self.contacts.is_empty() {
                        self.contact_index = Some(idx.min(self.contacts.len() - 1));
                    } else {
                        self.contact_index = None;
                    }
                    self.set_status("Contact deleted.");
                }
                Err(e) => self.set_error(&format!("delete contact: {e}")),
            }
        }
    }

    // ── Identities ────────────────────────────────────────────────────
}

impl App {
    /// Cycle the contact tag filter through "all" and every tag in use.
    pub(crate) fn cycle_contact_tag(&mut self) {
        let Some(ref conn) = self.db else {
            return;
        };
        let Ok(tags) = crate::contacts::all_tags(conn) else {
            return;
        };
        self.contact_tag_filter = match self.contact_tag_filter.as_deref() {
            None => tags.first().cloned(),
            Some(current) => {
                let position = tags.iter().position(|tag| tag == current);
                match position {
                    Some(index) if index + 1 < tags.len() => Some(tags[index + 1].clone()),
                    _ => None,
                }
            }
        };
        if let Some(ref conn) = self.db {
            match crate::contacts::list(conn, self.contact_tag_filter.as_deref()) {
                Ok(contacts) => {
                    self.contact_index = if contacts.is_empty() { None } else { Some(0) };
                    self.contacts = contacts;
                }
                Err(error) => {
                    self.set_error(&format!("contacts: {error}"));
                    return;
                }
            }
        }
        match self.contact_tag_filter.as_deref() {
            Some(tag) => self.set_status(&format!("Contacts tagged {tag}.")),
            None => self.set_status("All contacts."),
        }
    }
}
