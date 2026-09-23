/*! App state accessors and message rendering helpers. */
use crate::mail::types::Envelope;

use std::collections::HashMap;

use ratatui::widgets::TableState;

use super::model::{App, AUTO_REFRESH_TICKS};
use crate::db;
use crate::keys::View;
use crate::worker::Worker;

impl App {
    pub fn is_unified_inbox(&self) -> bool {
        self.current_folder == super::model::UNIFIED_INBOX
    }

    /// Set a transient status message (non-error).
    pub fn set_status(&mut self, msg: &str) {
        self.status_message = msg.to_string();
        self.status_is_error = false;
    }

    /// Set a transient error message.
    pub(crate) fn set_error(&mut self, msg: &str) {
        self.status_message = msg.to_string();
        self.status_is_error = true;
    }

    /// Owned account name for passing to worker threads.
    pub(crate) fn acct_owned(&self) -> Option<String> {
        self.account_name.clone()
    }

    /// Currently selected envelope ID, if any.
    pub fn selected_envelope_id(&self) -> Option<&str> {
        let idx = self.envelope_state.selected()?;
        self.envelopes.get(idx).map(|e| e.id.as_str())
    }

    /// Currently selected envelope, if any.
    pub fn selected_envelope(&self) -> Option<&Envelope> {
        let idx = self.envelope_state.selected()?;
        self.envelopes.get(idx)
    }
}

impl App {
    pub fn new(initial_account: Option<String>) -> Self {
        Self {
            running: true,
            view: View::EnvelopeList,
            previous_view: None,
            accounts: Vec::new(),
            account_index: 0,
            account_name: initial_account,
            folders: Vec::new(),
            folder_index: 0,
            current_folder: "INBOX".to_string(),
            triage_lane: None,
            followup_lane: None,
            pending_reply_marker: None,
            pending_bulk_to: None,
            conversation_anchors: HashMap::new(),
            muted_ids: std::collections::HashSet::new(),
            resurfaced_ids: std::collections::HashSet::new(),
            loud_ids: std::collections::HashSet::new(),
            resurface_input: String::new(),
            subject_aliases: HashMap::new(),
            annotation_input: String::new(),
            message_note: None,
            bypass_token: None,
            bypass_input: String::new(),
            cover_seen: false,
            cover_revealed: false,
            covered_count: 0,
            snippets: Default::default(),
            clips: Default::default(),
            stages: Default::default(),
            stage_of_anchor: HashMap::new(),
            stage_filter: None,
            collections: Default::default(),
            collection_of_anchor: HashMap::new(),
            collection_filter: None,
            merge_roots: HashMap::new(),
            pending_merge: None,
            bundled_senders: Default::default(),
            expanded_bundles: Default::default(),
            bundled_reps: Default::default(),
            focus: None,
            focus_reply_in_flight: false,
            envelopes: Vec::new(),
            envelope_state: TableState::default(),
            page: 1,
            message_content: None,
            message_scroll: 0,
            pgp_status: None,
            smime_signer: None,
            attachment_index: 0,
            folder_prompt: None,
            pending_folder_refresh: false,
            sieve: Default::default(),
            keys: Default::default(),
            saved_searches: Default::default(),
            show_all_headers: false,
            collapse_quotes: false,
            show_html_source: false,
            search_query: String::new(),
            active_query: None,
            search_scope: Default::default(),
            notification_rule: Default::default(),
            encrypt_drafts: false,
            planner123_calendar: None,
            mark_read_on_open: true,
            settings_index: 0,
            folder_jump: String::new(),
            sort_order: Default::default(),
            page_size: super::model::DEFAULT_PAGE_SIZE,
            autosave_seconds: 30,
            attachment_preview: None,
            preview_scroll: 0,
            schedule_input: String::new(),
            move_target: String::new(),
            move_index: 0,
            move_is_copy: false,
            crypto_passphrase: crate::mail::pgp::resolve_passphrase(),
            unlock_input: String::new(),
            threaded: false,
            ticks_since_refresh: 0,
            new_mail_count: 0,
            folder_unread: HashMap::new(),
            last_terminal_height: 24,
            last_terminal_width: 80,
            message_search: String::new(),
            message_search_active: false,
            message_matches: Vec::new(),
            message_match_index: 0,
            link_index: 0,
            help_scroll: 0,
            help_max_scroll: 0,
            status_message: String::new(),
            status_is_error: false,
            loading: false,
            tick_count: 0,
            pending_open_command: None,
            worker: Worker::new(),
            pending_message_id: None,
            pending_draft: None,
            pending_return_to_list: false,
            pending_refresh_after_action: false,
            selected: Default::default(),
            pending_undo: None,
            pending_empty_folder: None,
            pending_delete_account: None,
            collapsed_threads: std::collections::HashMap::new(),
            autosave_dir: super::autosave::default_dir(),
            autosave_ticks: 0,
            db: None,
            compose_state: None,
            contacts: Vec::new(),
            contact_index: None,
            contact_pending_delete: None,
            contact_search: String::new(),
            contact_search_active: false,
            contact_tag_filter: None,
            contact_edit_state: None,
            identities: Vec::new(),
            identity_index: None,
            account_edit_state: None,
            file_picker: None,
            outbox: Default::default(),
            attachment_library: Default::default(),
            read_together: None,
            read_together_scroll: 0,
            identity_edit_state: None,
        }
    }

    /// Initial startup: open the DB, recover an unsent message, load accounts.
    pub fn init(&mut self) {
        match db::open() {
            Ok(conn) => {
                self.db = Some(conn);
            }
            Err(e) => {
                self.set_error(&format!("DB error: {e}"));
            }
        }
        self.recover_autosave();
        self.load_preferences();
        self.loading = true;
        self.worker.fetch_accounts();
    }

    /// Handle a tick event: animate spinner + poll background results + auto-refresh.
    pub fn tick(&mut self) {
        self.tick_count = self.tick_count.wrapping_add(1);
        self.poll_worker();
        self.outbox_flush_tick();
        self.autosave_tick();

        // Auto-refresh: only when idle (not loading, on envelope list, page 1, no search)
        if !self.loading
            && self.view == View::EnvelopeList
            && self.page == 1
            && self.active_query.is_none()
            && self.compose_state.is_none()
        {
            self.ticks_since_refresh += 1;
            if self.ticks_since_refresh >= AUTO_REFRESH_TICKS {
                self.ticks_since_refresh = 0;
                self.load_envelopes();
            }
        }
    }
}
