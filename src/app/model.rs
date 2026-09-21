/*! Application state type and core accessors. */

use std::collections::HashMap;

use ratatui::widgets::TableState;
use rusqlite::Connection;

use crate::compose::ComposeState;
use crate::contact_edit::ContactEditState;
use crate::contacts::Contact;
use crate::db;
use crate::identities::Identity;
use crate::identity_edit::IdentityEditState;
use crate::keys::View;
use crate::mail::types::*;
use crate::mail::MessageDocument;
use crate::worker::Worker;

pub(crate) const PAGE_SIZE: usize = 50;

// Auto-refresh interval in ticks (250ms each). 240 ticks = 60 seconds.
const AUTO_REFRESH_TICKS: u64 = 240;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingOpenCommand {
    pub program: String,
    pub args: Vec<String>,
}

/// Top-level application state (TEA model).
pub struct App {
    pub running: bool,

    // ── View state ──────────────────────────────────────────────────
    pub view: View,
    pub previous_view: Option<View>,

    // ── Account state ───────────────────────────────────────────────
    pub accounts: Vec<Account>,
    pub account_index: usize,
    pub account_name: Option<String>,

    // ── Folder state ────────────────────────────────────────────────
    pub folders: Vec<Folder>,
    pub folder_index: usize,
    pub current_folder: String,

    // ── Envelope state ──────────────────────────────────────────────
    pub envelopes: Vec<Envelope>,
    pub envelope_state: TableState,
    pub page: usize,

    // ── Message view state ──────────────────────────────────────────
    pub message_content: Option<MessageDocument>,
    pub message_scroll: u16,
    /// PGP verification/decryption result for the current message.
    pub pgp_status: Option<String>,

    // ── Search state ────────────────────────────────────────────────
    pub search_query: String,
    pub active_query: Option<String>,

    // ── Move prompt state ───────────────────────────────────────────
    pub move_target: String,

    // ── Help scroll ─────────────────────────────────────────────────
    pub help_scroll: u16,

    // ── Status / loading ────────────────────────────────────────────
    pub status_message: String,
    pub status_is_error: bool,
    pub loading: bool,
    pub tick_count: u64,

    // ── Threading mode ───────────────────────────────────────────────
    pub threaded: bool,

    // ── Auto-refresh ────────────────────────────────────────────────
    pub(crate) ticks_since_refresh: u64,
    pub new_mail_count: usize,

    // ── Folder unread counts ────────────────────────────────────────
    pub folder_unread: HashMap<String, usize>,

    // ── Layout areas for mouse hit-testing ──────────────────────────
    pub last_terminal_height: u16,

    // ── Shell-out command ───────────────────────────────────────────
    pub pending_open_command: Option<PendingOpenCommand>,

    // ── Background worker ───────────────────────────────────────────
    pub(crate) worker: Worker,

    // ── Pending state for message view after background load ────────
    pub(crate) pending_message_id: Option<String>,

    // ── Draft being resumed (folder, id), deleted after a successful send ──
    pub(crate) pending_draft: Option<(String, String)>,

    // ── Track if delete/move was from message view ──────────────────
    pub(crate) pending_return_to_list: bool,
    pub(crate) pending_refresh_after_action: bool,

    // ── Database ────────────────────────────────────────────────────
    pub db: Option<Connection>,

    // ── Compose editor state ─────────────────────────────────────────
    pub compose_state: Option<ComposeState>,

    // ── Contacts browser state ───────────────────────────────────────
    pub contacts: Vec<Contact>,
    pub contact_index: Option<usize>,
    pub contact_search: String,
    /// Whether the contacts search bar is active (accepting typed characters).
    pub contact_search_active: bool,

    // ── Contact edit form state ──────────────────────────────────────
    pub contact_edit_state: Option<ContactEditState>,

    // ── Identity list state ──────────────────────────────────────────
    pub identities: Vec<Identity>,
    pub identity_index: Option<usize>,

    // ── Identity edit form state ─────────────────────────────────────
    pub identity_edit_state: Option<IdentityEditState>,
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
            envelopes: Vec::new(),
            envelope_state: TableState::default(),
            page: 1,
            message_content: None,
            message_scroll: 0,
            pgp_status: None,
            search_query: String::new(),
            active_query: None,
            move_target: String::new(),
            threaded: false,
            ticks_since_refresh: 0,
            new_mail_count: 0,
            folder_unread: HashMap::new(),
            last_terminal_height: 24,
            help_scroll: 0,
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
            db: None,
            compose_state: None,
            contacts: Vec::new(),
            contact_index: None,
            contact_search: String::new(),
            contact_search_active: false,
            contact_edit_state: None,
            identities: Vec::new(),
            identity_index: None,
            identity_edit_state: None,
        }
    }

    /// Initial startup: open the DB and load accounts.
    pub fn init(&mut self) {
        match db::open() {
            Ok(conn) => {
                self.db = Some(conn);
            }
            Err(e) => {
                self.set_error(&format!("DB error: {e}"));
            }
        }
        self.loading = true;
        self.worker.fetch_accounts();
    }

    /// Handle a tick event: animate spinner + poll background results + auto-refresh.
    pub fn tick(&mut self) {
        self.tick_count = self.tick_count.wrapping_add(1);
        self.poll_worker();

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

    pub fn current_message(&self) -> Option<&MessageDocument> {
        self.message_content.as_ref()
    }

    pub fn render_message_body(&self, width: usize) -> String {
        self.current_message()
            .map(|message| message.render(width))
            .unwrap_or_default()
    }

    pub fn rendered_message_line_count(&self, width: usize) -> u16 {
        let header_lines = self
            .current_message()
            .map(|message| {
                let attachment_lines = if message.attachments.is_empty() {
                    0
                } else {
                    message.attachments.len() as u16 + 2
                };
                message.header_fields().len() as u16 + 3 + attachment_lines
            })
            .unwrap_or(0);
        let body_lines = self.render_message_body(width).lines().count() as u16;
        header_lines.saturating_add(body_lines)
    }

    // ── Background result polling ───────────────────────────────────
}
