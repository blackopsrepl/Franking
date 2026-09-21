/*! Application state type and core accessors. */

use std::collections::HashMap;

use ratatui::widgets::TableState;
use rusqlite::Connection;

use crate::compose::ComposeState;
use crate::contact_edit::ContactEditState;
use crate::contacts::Contact;
use crate::identities::Identity;
use crate::identity_edit::IdentityEditState;
use crate::keys::View;
use crate::mail::types::*;
use crate::mail::MessageDocument;
use crate::worker::Worker;

pub(crate) const PAGE_SIZE: usize = 50;

pub(crate) const AUTO_REFRESH_TICKS: u64 = 240;

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

    pub(crate) show_all_headers: bool,
    pub(crate) collapse_quotes: bool,
    /// Whether the message view shows the raw HTML source.
    pub(crate) show_html_source: bool,

    pub search_query: String,
    pub active_query: Option<String>,
    pub(crate) search_all_folders: bool,
    /// Whether new-mail desktop notifications are sent.
    pub(crate) notifications_enabled: bool,
    /// Whether opening a message marks it read on the server.
    pub(crate) mark_read_on_open: bool,
    /// Highlighted preference row.
    pub(crate) settings_index: usize,
    /// Incremental folder-search query.
    pub(crate) folder_jump: String,
    /// Buffered send-later delay input.
    pub(crate) schedule_input: String,
    /// Invitation awaiting a response choice.
    pub(crate) invite_pending: Option<crate::mail::calendar::Event>,

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
    /// Active contact tag filter.
    pub(crate) contact_tag_filter: Option<String>,

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
