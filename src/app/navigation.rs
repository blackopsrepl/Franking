/*! View navigation, search, and prompt handling. */

use crate::keys::View;

use super::model::App;

impl App {
    pub(crate) fn go_back(&mut self) {
        match self.view {
            View::MessageView => {
                self.view = View::EnvelopeList;
                self.message_content = None;
            }
            View::AccountList => {
                self.view = View::EnvelopeList;
            }
            View::Contacts | View::ContactSearch => {
                self.contact_search.clear();
                self.contact_search_active = false;
                self.view = self.previous_view.unwrap_or(View::EnvelopeList);
                self.previous_view = None;
            }
            View::Compose => {
                // Use compose_discard logic
                self.compose_discard();
            }
            View::ContactEdit => {
                self.contact_edit_cancel();
            }
            View::IdentityList => {
                self.identity_list_close();
            }
            View::IdentityEdit => {
                self.identity_edit_cancel();
            }
            _ => {}
        }
    }

    pub(crate) fn move_selection(&mut self, delta: i32) {
        match self.view {
            View::EnvelopeList => {
                let len = self.envelopes.len();
                if len == 0 {
                    return;
                }
                let current = self.envelope_state.selected().unwrap_or(0);
                let next = if delta > 0 {
                    (current + 1).min(len - 1)
                } else {
                    current.saturating_sub(1)
                };
                self.envelope_state.select(Some(next));
            }
            View::FolderList => {
                let len = self.folders.len();
                if len == 0 {
                    return;
                }
                if delta > 0 {
                    self.folder_index = (self.folder_index + 1).min(len - 1);
                } else {
                    self.folder_index = self.folder_index.saturating_sub(1);
                }
            }
            View::AccountList => {
                let len = self.accounts.len();
                if len == 0 {
                    return;
                }
                if delta > 0 {
                    self.account_index = (self.account_index + 1).min(len - 1);
                } else {
                    self.account_index = self.account_index.saturating_sub(1);
                }
            }
            View::Contacts => {
                let len = self.contacts.len();
                if len == 0 {
                    return;
                }
                let current = self.contact_index.unwrap_or(0);
                let next = if delta > 0 {
                    (current + 1).min(len - 1)
                } else {
                    current.saturating_sub(1)
                };
                self.contact_index = Some(next);
            }
            _ => {}
        }
    }

    pub(crate) fn jump_top(&mut self) {
        match self.view {
            View::EnvelopeList => {
                if !self.envelopes.is_empty() {
                    self.envelope_state.select(Some(0));
                }
            }
            View::MessageView | View::Help => {
                if self.view == View::Help {
                    self.help_scroll = 0;
                } else {
                    self.message_scroll = 0;
                }
            }
            _ => {}
        }
    }

    pub(crate) fn jump_bottom(&mut self) {
        match self.view {
            View::EnvelopeList => {
                if !self.envelopes.is_empty() {
                    self.envelope_state.select(Some(self.envelopes.len() - 1));
                }
            }
            View::MessageView => {
                let lines = self.rendered_message_line_count(78);
                self.message_scroll = lines.saturating_sub(5);
            }
            View::Help => {
                self.help_scroll = 100; // will be clamped in render
            }
            _ => {}
        }
    }

    pub(crate) fn page_up(&mut self) {
        if self.view == View::EnvelopeList && self.page > 1 {
            self.page -= 1;
            self.load_envelopes();
        }
    }

    pub(crate) fn page_down(&mut self) {
        if self.view == View::EnvelopeList && self.envelopes.len() >= self.page_size {
            self.page += 1;
            self.load_envelopes();
        }
    }

    pub(crate) fn select_item(&mut self) {
        match self.view {
            View::FolderList => {
                if let Some(folder) = self.folders.get(self.folder_index) {
                    self.current_folder = folder.name.clone();
                    self.page = 1;
                    self.active_query = None;
                    self.view = View::EnvelopeList;
                    self.load_envelopes();
                }
            }
            View::AccountList => {
                if let Some(account) = self.accounts.get(self.account_index) {
                    self.account_name = Some(account.name.clone());
                    self.current_folder = "INBOX".to_string();
                    self.page = 1;
                    self.active_query = None;
                    self.view = View::EnvelopeList;
                    self.load_folders();
                }
            }
            _ => {}
        }
    }

    pub(crate) fn scroll(&mut self, delta: i32) {
        match self.view {
            View::MessageView => {
                if delta > 0 {
                    self.message_scroll = self.message_scroll.saturating_add(1);
                } else {
                    self.message_scroll = self.message_scroll.saturating_sub(1);
                }
            }
            View::Help => {
                if delta > 0 {
                    self.help_scroll = self.help_scroll.saturating_add(1);
                } else {
                    self.help_scroll = self.help_scroll.saturating_sub(1);
                }
            }
            _ => {}
        }
    }

    /// Load sender identities from the DB into a ComposeState.
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
        let query = self.search_query.clone();
        self.active_query = if query.is_empty() { None } else { Some(query) };
        self.page = 1;
        let scope = self.search_scope;
        let query = self.active_query.clone();
        self.view = View::EnvelopeList;

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

    pub(crate) fn refresh(&mut self) {
        self.ticks_since_refresh = 0;
        self.new_mail_count = 0;
        self.load_folders();
        // load_envelopes will be chained after folders complete
    }

    /// Toggle the message view between the header summary and every header.
    pub(crate) fn toggle_headers(&mut self) {
        self.show_all_headers = !self.show_all_headers;
    }

    /// Collapse or expand quoted lines in the message view.
    pub(crate) fn toggle_quotes(&mut self) {
        self.collapse_quotes = !self.collapse_quotes;
    }

    pub(crate) fn enter_account_picker(&mut self) {
        self.load_accounts();
        self.view = View::AccountList;
    }

    pub(crate) fn toggle_help(&mut self) {
        if self.view == View::Help {
            self.view = self.previous_view.unwrap_or(View::EnvelopeList);
            self.previous_view = None;
        } else {
            self.previous_view = Some(self.view);
            self.help_scroll = 0;
            self.view = View::Help;
        }
    }

    // ── Editor key forwarding ────────────────────────────────────────
}

impl App {
    /// Toggle between the rendered message text and the original HTML source.
    pub(crate) fn toggle_html_source(&mut self) {
        let has_html = self
            .message_content
            .as_ref()
            .and_then(|message| message.html_body.as_deref())
            .is_some_and(|html| !html.trim().is_empty());
        if !has_html {
            self.show_html_source = false;
            self.set_status("This message has no HTML part.");
            return;
        }
        self.show_html_source = !self.show_html_source;
        self.message_scroll = 0;
    }
}
