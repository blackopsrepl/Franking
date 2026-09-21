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

const AUTO_REFRESH_TICKS: u64 = 240;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingOpenCommand {
    pub program: String,
    pub args: Vec<String>,
}

/// Synthetic folder aggregating every account's inbox.
pub(crate) const UNIFIED_INBOX: &str = "All Inboxes";

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
    /// S/MIME signer awaiting a trust decision for the current message.
    pub(crate) smime_signer: Option<crate::mail::smime::SmimeSigner>,

    // ── Attachment list ─────────────────────────────────────────────
    /// Selected attachment index while the attachment list is open.
    pub(crate) attachment_index: usize,

    /// Pending folder-management prompt, if open.
    pub(crate) folder_prompt: Option<super::folders::FolderPrompt>,
    /// Whether to reload the folder list after the next action completes.
    pub(crate) pending_folder_refresh: bool,
    /// Server-side Sieve script browser and editor state.
    pub(crate) sieve: super::sieve::SieveState,

    /// Whether the message view shows every raw header.
    pub(crate) show_all_headers: bool,

    // ── Search state ────────────────────────────────────────────────
    pub search_query: String,
    pub active_query: Option<String>,

    // ── Move prompt state ───────────────────────────────────────────
    pub move_target: String,
    /// Highlighted row in the move-to-folder picker.
    pub(crate) move_index: usize,
    /// Whether the folder picker copies instead of moving.
    pub(crate) move_is_copy: bool,

    // ── Crypto unlock state ─────────────────────────────────────────
    /// Passphrase for PGP secret keys, seeded from the environment or the
    /// keys directory and replaceable through the unlock prompt.
    pub(crate) crypto_passphrase: String,
    /// Buffered input while the passphrase prompt is open.
    pub(crate) unlock_input: String,

    // ── Help scroll ─────────────────────────────────────────────────
    pub help_scroll: u16,

    // ── Status / loading ────────────────────────────────────────────
    pub status_message: String,
    pub status_is_error: bool,
    pub loading: bool,
    pub tick_count: u64,

    // ── Threading mode ───────────────────────────────────────────────
    pub threaded: bool,

    pub(crate) ticks_since_refresh: u64,
    pub new_mail_count: usize,

    pub folder_unread: HashMap<String, usize>,

    pub last_terminal_height: u16,
    /// Width of the last rendered frame, used to render the message body.
    pub(crate) last_terminal_width: u16,
    /// Active in-message search query.
    pub(crate) message_search: String,
    /// Whether the in-message search prompt is open.
    pub(crate) message_search_active: bool,
    /// Line offsets of the current matches within the rendered message.
    pub(crate) message_matches: Vec<u16>,
    /// Highlighted match index.
    pub(crate) message_match_index: usize,
    /// Selected row in the message link list.
    pub(crate) link_index: usize,

    // ── Shell-out command ───────────────────────────────────────────
    pub pending_open_command: Option<PendingOpenCommand>,

    // ── Background worker ───────────────────────────────────────────
    pub(crate) worker: Worker,

    // ── Pending state for message view after background load ────────
    pub(crate) pending_message_id: Option<String>,

    // ── Draft being resumed (folder, id), deleted after a successful send ──
    pub(crate) pending_draft: Option<(String, String)>,

    pub(crate) pending_return_to_list: bool,
    pub(crate) pending_refresh_after_action: bool,
    /// Envelope ids selected for a batch operation.
    pub(crate) selected: std::collections::HashSet<String>,
    /// Reversible operation performed by the last destructive action.
    pub(crate) pending_undo: Option<super::undo::UndoOp>,

    /// Folder awaiting a second press of the empty-folder key.
    pub(crate) pending_empty_folder: Option<String>,
    /// Account awaiting a second press of the delete key.
    pub(crate) pending_delete_account: Option<String>,
    /// Envelopes hidden by collapsing a thread, keyed by thread root.
    pub(crate) collapsed_threads: std::collections::HashMap<String, Vec<Envelope>>,

    /// Directory holding the crash-safe autosave of the message in progress.
    pub(crate) autosave_dir: std::path::PathBuf,
    /// Ticks since the last autosave was written.
    pub(crate) autosave_ticks: u64,

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

    /// Account add/edit form state.
    pub account_edit_state: Option<crate::account_edit::AccountEditState>,
    /// Filesystem picker used to choose an attachment path.
    pub(crate) file_picker: Option<crate::file_picker::FilePickerState>,

    /// Queued messages and send state.
    pub(crate) outbox: super::outbox::OutboxState,
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
            smime_signer: None,
            attachment_index: 0,
            folder_prompt: None,
            pending_folder_refresh: false,
            sieve: Default::default(),
            show_all_headers: false,
            search_query: String::new(),
            active_query: None,
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
            contact_search: String::new(),
            contact_search_active: false,
            contact_edit_state: None,
            identities: Vec::new(),
            identity_index: None,
            account_edit_state: None,
            file_picker: None,
            outbox: Default::default(),
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
        self.loading = true;
        self.worker.fetch_accounts();
    }

    /// Handle a tick event: animate spinner + poll background results + auto-refresh.
    pub fn tick(&mut self) {
        self.tick_count = self.tick_count.wrapping_add(1);
        self.poll_worker();
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
