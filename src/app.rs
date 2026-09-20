use std::collections::HashMap;

use crossterm::event::{KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use ratatui::widgets::TableState;
use rusqlite::Connection;

use crate::compose::{populate_from_template, ComposeMode, ComposeState, FocusedField};
use crate::contact_edit::ContactEditState;
use crate::contacts::Contact;
use crate::db;
use crate::identities::Identity;
use crate::identity_edit::IdentityEditState;
use crate::keys::EditMode;
use crate::keys::{self, Action, ComposeFocus, ComposeKeyContext, View};
use crate::mail::types::*;
use crate::mail::{MessageContent, MessageDisplayMode};
use crate::worker::{Worker, WorkerResult};

// Page size for envelope listing.
const PAGE_SIZE: usize = 50;

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
    pub message_content: Option<MessageContent>,
    pub message_display_mode: MessageDisplayMode,
    pub message_scroll: u16,

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
    ticks_since_refresh: u64,
    pub new_mail_count: usize,

    // ── Folder unread counts ────────────────────────────────────────
    pub folder_unread: HashMap<String, usize>,

    // ── Layout areas for mouse hit-testing ──────────────────────────
    pub last_terminal_height: u16,

    // ── Shell-out command ───────────────────────────────────────────
    pub pending_open_command: Option<PendingOpenCommand>,

    // ── Background worker ───────────────────────────────────────────
    worker: Worker,

    // ── Pending state for message view after background load ────────
    pending_message_id: Option<String>,

    // ── Track if delete/move was from message view ──────────────────
    pending_return_to_list: bool,
    pending_refresh_after_action: bool,

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
            message_display_mode: MessageDisplayMode::Auto,
            message_scroll: 0,
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
    fn set_error(&mut self, msg: &str) {
        self.status_message = msg.to_string();
        self.status_is_error = true;
    }

    /// Owned account name for passing to worker threads.
    fn acct_owned(&self) -> Option<String> {
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

    pub fn current_message(&self) -> Option<&MessageContent> {
        self.message_content.as_ref()
    }

    pub fn resolved_message_display_mode(&self) -> MessageDisplayMode {
        self.current_message()
            .map(|message| message.resolve_display_mode(self.message_display_mode))
            .unwrap_or(MessageDisplayMode::Auto)
    }

    pub fn render_message_body(&self, width: usize) -> String {
        self.current_message()
            .map(|message| message.render_body(MessageDisplayMode::Auto, width))
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
                message.headers.len() as u16 + 3 + attachment_lines
            })
            .unwrap_or(0);
        let body_lines = self.render_message_body(width).lines().count() as u16;
        header_lines.saturating_add(body_lines)
    }

    // ── Background result polling ───────────────────────────────────

    fn poll_worker(&mut self) {
        for result in self.worker.drain() {
            match result {
                WorkerResult::Accounts(Ok(accounts)) => {
                    self.handle_accounts_loaded(accounts);
                }
                WorkerResult::Accounts(Err(e)) => {
                    self.loading = false;
                    self.set_error(&format!("Failed to load accounts: {e}"));
                }
                WorkerResult::Folders(Ok(folders)) => {
                    self.handle_folders_loaded(folders);
                }
                WorkerResult::Folders(Err(e)) => {
                    self.loading = false;
                    self.set_error(&format!("Failed to load folders: {e}"));
                }
                WorkerResult::Envelopes(Ok(envelopes)) => {
                    self.handle_envelopes_loaded(envelopes);
                }
                WorkerResult::Envelopes(Err(e)) => {
                    self.loading = false;
                    self.set_error(&format!("Failed to load envelopes: {e}"));
                }
                WorkerResult::Message(Ok(message)) => {
                    self.handle_message_loaded(message);
                }
                WorkerResult::Message(Err(e)) => {
                    self.loading = false;
                    self.set_error(&format!("Failed to read message: {e}"));
                }
                WorkerResult::ActionDone(Ok(msg)) => {
                    self.loading = false;
                    self.set_status(&msg);
                    if self.pending_return_to_list {
                        self.view = View::EnvelopeList;
                        self.pending_return_to_list = false;
                    }
                    if self.pending_refresh_after_action {
                        self.pending_refresh_after_action = false;
                        self.load_envelopes();
                    }
                }
                WorkerResult::ActionDone(Err(e)) => {
                    self.loading = false;
                    self.set_error(&format!("Error: {e}"));
                    self.pending_return_to_list = false;
                    self.pending_refresh_after_action = false;
                }
                WorkerResult::FolderUnread(folder_name, Ok(count)) => {
                    self.folder_unread.insert(folder_name, count);
                }
                WorkerResult::FolderUnread(_folder_name, Err(_)) => {
                    // Silently ignore unread count failures
                }
                WorkerResult::Template(Ok(template)) => {
                    self.loading = false;
                    self.handle_template_loaded(template);
                }
                WorkerResult::Template(Err(e)) => {
                    self.loading = false;
                    self.set_error(&format!("Template error: {e}"));
                    self.compose_state = None;
                }
                WorkerResult::SendDone(Ok(msg)) => {
                    self.loading = false;
                    self.compose_state = None;
                    self.view = View::EnvelopeList;
                    self.set_status(&msg);
                    self.refresh_envelopes();
                }
                WorkerResult::SendDone(Err(e)) => {
                    self.loading = false;
                    if let Some(ref mut cs) = self.compose_state {
                        cs.send_error = Some(e.to_string());
                    }
                }
            }
        }
    }

    fn handle_template_loaded(&mut self, raw: String) {
        if let Some(ref mut cs) = self.compose_state {
            populate_from_template(cs, &raw);
            self.view = View::Compose;
        }
    }

    fn handle_accounts_loaded(&mut self, accounts: Vec<Account>) {
        // If no account was specified on the CLI, use the default.
        if self.account_name.is_none() {
            if let Some(account) = preferred_account(&accounts) {
                self.account_name = Some(account.name.clone());
            }
        }
        // Set account_index to match account_name
        if let Some(name) = &self.account_name {
            self.account_index = accounts.iter().position(|a| &a.name == name).unwrap_or(0);
        }
        self.accounts = accounts;
        // Chain: after accounts, load folders
        self.load_folders();
    }

    fn handle_folders_loaded(&mut self, folders: Vec<Folder>) {
        self.folders = folders;
        // Reset folder index to match current_folder
        self.folder_index = self
            .folders
            .iter()
            .position(|f| f.name == self.current_folder)
            .unwrap_or(0);
        // Fire off background unread count queries for each folder
        for folder in &self.folders {
            self.worker
                .fetch_folder_unread(self.acct_owned(), folder.name.clone());
        }
        // Chain: after folders, load envelopes
        self.load_envelopes();
    }

    fn handle_envelopes_loaded(&mut self, envelopes: Vec<Envelope>) {
        // Detect new mail by comparing unseen counts
        let old_unseen: usize = self.envelopes.iter().filter(|e| !e.is_seen()).count();
        let new_unseen: usize = envelopes.iter().filter(|e| !e.is_seen()).count();

        let was_populated = !self.envelopes.is_empty();
        let selection = self.envelope_state.selected();

        self.envelopes = envelopes;

        if !self.envelopes.is_empty() {
            // Preserve selection position on auto-refresh if possible
            if was_populated {
                let idx = selection.unwrap_or(0).min(self.envelopes.len() - 1);
                self.envelope_state.select(Some(idx));
            } else {
                self.envelope_state.select(Some(0));
            }
        } else {
            self.envelope_state.select(None);
        }

        self.loading = false;
        self.ticks_since_refresh = 0;

        if was_populated && new_unseen > old_unseen {
            let diff = new_unseen - old_unseen;
            self.new_mail_count = diff;
            self.set_status(&format!(
                "{diff} new message{}.",
                if diff == 1 { "" } else { "s" }
            ));
        } else if self.envelopes.is_empty() {
            self.set_status("No messages.");
        } else if !was_populated {
            // Initial load, don't set "new mail" status
            self.status_message.clear();
        }
    }

    fn handle_message_loaded(&mut self, message: MessageContent) {
        self.harvest_contacts_from_message(&message);

        self.message_content = Some(message);
        self.message_display_mode = MessageDisplayMode::Auto;
        self.message_scroll = 0;
        self.loading = false;
        self.view = View::MessageView;
        // Mark as seen in local state
        if let Some(idx) = self.envelope_state.selected() {
            if let Some(env) = self.envelopes.get_mut(idx) {
                if !env.is_seen() {
                    env.flags.push("Seen".to_string());
                }
            }
        }
    }

    /// Parse From/To/Cc/Reply-To addresses from message headers and upsert them
    /// into the contacts DB. Errors are silently ignored (harvest is
    /// best-effort).
    fn harvest_contacts_from_message(&mut self, message: &MessageContent) {
        if self.db.is_none() {
            return;
        }

        let mut addrs: Vec<(Option<String>, String)> = Vec::new();

        for header in &message.headers {
            let is_addr_header = header.name.eq_ignore_ascii_case("from")
                || header.name.eq_ignore_ascii_case("to")
                || header.name.eq_ignore_ascii_case("cc")
                || header.name.eq_ignore_ascii_case("reply-to");
            if is_addr_header {
                addrs.extend(crate::contacts::parse_address_list(&header.value));
            }
        }

        // Also harvest the envelope sender directly from the already-parsed list row.
        // Collect separately to avoid holding a borrow on self while calling upsert.
        let sender_str = self
            .selected_envelope()
            .map(|e| e.sender.display())
            .unwrap_or_default();
        if !sender_str.is_empty() {
            addrs.extend(crate::contacts::parse_address_list(&sender_str));
        }

        // Now borrow the connection and upsert all collected addresses.
        if let Some(ref conn) = self.db {
            for (name, email) in addrs {
                let _ = crate::contacts::upsert_harvested(conn, name.as_deref(), &email);
            }
        }
    }

    // ── Data loading (dispatches to worker) ─────────────────────────

    fn load_accounts(&mut self) {
        self.loading = true;
        self.worker.fetch_accounts();
    }

    fn load_folders(&mut self) {
        self.loading = true;
        self.worker.fetch_folders(self.acct_owned());
    }

    fn load_envelopes(&mut self) {
        self.loading = true;
        if self.threaded {
            self.worker.fetch_envelopes_threaded(
                self.acct_owned(),
                self.current_folder.clone(),
                self.active_query.clone(),
            );
        } else {
            self.worker.fetch_envelopes(
                self.acct_owned(),
                self.current_folder.clone(),
                self.page,
                PAGE_SIZE,
                self.active_query.clone(),
            );
        }
    }

    pub fn refresh_envelopes(&mut self) {
        self.load_envelopes();
    }

    fn load_message(&mut self) {
        if let Some(id) = self.selected_envelope_id().map(|s| s.to_string()) {
            self.loading = true;
            self.pending_message_id = Some(id.clone());
            self.worker
                .fetch_message(self.acct_owned(), self.current_folder.clone(), id);
        }
    }

    // ── Mouse handling ───────────────────────────────────────────────

    pub fn handle_mouse(&mut self, mouse: MouseEvent) {
        // Layout: header (row 0), body (rows 1..h-1), status (row h-1)
        // Body: sidebar cols 0..22, envelope list cols 23..
        // Message view: full width (no sidebar)
        let h = self.last_terminal_height;
        let row = mouse.row;
        let col = mouse.column;

        match mouse.kind {
            MouseEventKind::ScrollUp => match self.view {
                View::MessageView => self.scroll(-3),
                View::Help => self.scroll(-3),
                View::EnvelopeList => {
                    for _ in 0..3 {
                        self.move_selection(-1);
                    }
                }
                View::FolderList => self.move_selection(-1),
                _ => {}
            },
            MouseEventKind::ScrollDown => match self.view {
                View::MessageView => self.scroll(3),
                View::Help => self.scroll(3),
                View::EnvelopeList => {
                    for _ in 0..3 {
                        self.move_selection(1);
                    }
                }
                View::FolderList => self.move_selection(1),
                _ => {}
            },
            MouseEventKind::Down(MouseButton::Left) => {
                if row == 0 || row >= h.saturating_sub(1) {
                    // Click on header or status bar -- ignore
                    return;
                }

                match self.view {
                    View::MessageView | View::Help | View::AccountList => {
                        // In overlays, clicks don't do navigation
                    }
                    _ => {
                        // Body area
                        let body_row = (row - 1) as usize; // offset past header

                        if col < 22 {
                            // Sidebar click
                            if body_row > 0 && body_row <= self.folders.len() {
                                // Account for border (row 0 of sidebar is border)
                                let folder_idx = body_row.saturating_sub(1);
                                if folder_idx < self.folders.len() {
                                    self.folder_index = folder_idx;
                                    self.view = View::FolderList;
                                    self.select_item();
                                }
                            }
                        } else {
                            // Envelope list click
                            // Account for border + header row (2 rows of table overhead)
                            if body_row >= 2 {
                                let env_idx = body_row.saturating_sub(2);
                                if env_idx < self.envelopes.len() {
                                    self.envelope_state.select(Some(env_idx));
                                    self.view = View::EnvelopeList;
                                }
                            }
                        }
                    }
                }
            }
            // Right-click to go back
            MouseEventKind::Down(MouseButton::Right) if self.view == View::MessageView => {
                self.go_back()
            }
            _ => {}
        }
    }

    // ── Key handling ────────────────────────────────────────────────

    fn compose_key_context(&self) -> ComposeKeyContext {
        self.compose_state
            .as_ref()
            .map(|cs| ComposeKeyContext {
                focus: match cs.focused {
                    FocusedField::From => ComposeFocus::From,
                    FocusedField::To
                    | FocusedField::Cc
                    | FocusedField::Bcc
                    | FocusedField::Subject => ComposeFocus::Header,
                    FocusedField::Body => ComposeFocus::Body,
                    FocusedField::Send
                    | FocusedField::Draft
                    | FocusedField::Attach
                    | FocusedField::Discard => ComposeFocus::ActionBar,
                },
                edit_mode: cs.edit_mode,
                body_search_active: cs.body.is_search_active(),
                autocomplete_visible: cs.autocomplete.is_some(),
                confirm_discard_visible: cs.confirm_discard,
            })
            .unwrap_or(ComposeKeyContext {
                focus: ComposeFocus::Header,
                edit_mode: EditMode::Nav,
                body_search_active: false,
                autocomplete_visible: false,
                confirm_discard_visible: false,
            })
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        let action = if self.view == View::Compose {
            let ctx = self.compose_key_context();
            keys::resolve_compose_with_context(key, ctx)
        } else {
            keys::resolve(self.view, key)
        };

        match action {
            Action::Quit => self.running = false,
            Action::Back => self.go_back(),
            Action::MoveUp => self.move_selection(-1),
            Action::MoveDown => self.move_selection(1),
            Action::PageUp => self.page_up(),
            Action::PageDown => self.page_down(),
            Action::JumpTop => self.jump_top(),
            Action::JumpBottom => self.jump_bottom(),
            Action::Select => self.select_item(),
            Action::OpenMessage => self.load_message(),
            Action::Compose => self.compose(),
            Action::Reply => self.reply(false),
            Action::ReplyAll => self.reply(true),
            Action::Forward => self.forward(),
            Action::Delete => self.delete(),
            Action::ToggleFlag => self.toggle_flag(),
            Action::DownloadAttachments => self.download_attachments(),
            Action::ToggleThread => self.toggle_thread(),
            Action::Search => self.enter_search(),
            Action::SearchSubmit => self.submit_search(),
            Action::SearchCancel => self.cancel_search(),
            Action::SearchInput(c) => self.search_query.push(c),
            Action::SearchBackspace => {
                self.search_query.pop();
            }
            Action::Refresh => self.refresh(),
            Action::SwitchAccount => self.enter_account_picker(),
            Action::ToggleHelp => self.toggle_help(),
            Action::FocusFolders => self.view = View::FolderList,
            Action::FocusEnvelopes => self.view = View::EnvelopeList,
            Action::ScrollUp => self.scroll(-1),
            Action::ScrollDown => self.scroll(1),
            Action::MoveMessage => self.enter_move_prompt(),
            Action::MoveInput(c) => self.move_target.push(c),
            Action::MoveBackspace => {
                self.move_target.pop();
            }
            Action::MoveSubmit => self.submit_move(),
            Action::MoveCancel => self.cancel_move(),

            // ── Compose editor ───────────────────────────────────────
            Action::ComposeFieldNext => {
                if let Some(ref mut cs) = self.compose_state {
                    cs.autocomplete = None;
                    cs.focused = cs.focused.next();
                }
            }
            Action::ComposeFieldPrev => {
                if let Some(ref mut cs) = self.compose_state {
                    cs.autocomplete = None;
                    cs.focused = cs.focused.prev();
                }
            }
            Action::ComposeLeaveBodyNext => {
                if let Some(ref mut cs) = self.compose_state {
                    cs.body.clear_search();
                    cs.autocomplete = None;
                    cs.focused = cs.focused.next();
                }
            }
            Action::ComposeLeaveBodyPrev => {
                if let Some(ref mut cs) = self.compose_state {
                    cs.body.clear_search();
                    cs.autocomplete = None;
                    cs.focused = cs.focused.prev();
                }
            }
            Action::ComposeSend => self.compose_send(),
            Action::ComposeDiscard => self.compose_discard(),
            Action::ComposeConfirmDiscard => {
                self.compose_state = None;
                self.view = View::EnvelopeList;
            }
            Action::ComposeCancelDiscard => {
                if let Some(ref mut cs) = self.compose_state {
                    cs.confirm_discard = false;
                }
            }
            Action::ComposeInput(c) => {
                if let Some(ref mut cs) = self.compose_state {
                    // Auto-enter Insert on header text fields when typing.
                    if cs.edit_mode == EditMode::Nav
                        && matches!(
                            cs.focused,
                            FocusedField::To
                                | FocusedField::Cc
                                | FocusedField::Bcc
                                | FocusedField::Subject
                        )
                    {
                        cs.edit_mode = EditMode::Insert;
                    }

                    if cs.edit_mode == EditMode::Insert {
                        if let Some(field) = cs.focused_line_field_mut() {
                            field.push(c);
                            cs.dirty = true;
                        }
                    }
                }
                let is_address = {
                    let cs = self.compose_state.as_ref();
                    cs.map(|cs| {
                        cs.edit_mode == EditMode::Insert
                            && matches!(
                                cs.focused,
                                FocusedField::To | FocusedField::Cc | FocusedField::Bcc
                            )
                    })
                    .unwrap_or(false)
                };
                if is_address {
                    self.update_autocomplete();
                }
            }
            Action::ComposeBackspace => {
                let is_address = {
                    let cs = self.compose_state.as_ref();
                    cs.map(|cs| {
                        cs.edit_mode == EditMode::Insert
                            && matches!(
                                cs.focused,
                                FocusedField::To | FocusedField::Cc | FocusedField::Bcc
                            )
                    })
                    .unwrap_or(false)
                };
                if let Some(ref mut cs) = self.compose_state {
                    if cs.edit_mode == EditMode::Insert {
                        if let Some(field) = cs.focused_line_field_mut() {
                            field.pop();
                        }
                    }
                }
                if is_address {
                    self.update_autocomplete();
                }
            }
            Action::ComposeEnterInsert => {
                self.compose_enter_insert();
            }
            Action::ComposeExitToNav => {
                self.compose_exit_to_nav();
            }

            // ── EditorKey: forwarded to the focused compose field ────
            Action::EditorKey(key_event) => {
                self.handle_editor_key(key_event);
            }

            // ── Contacts ─────────────────────────────────────────────
            Action::OpenContacts => self.open_contacts(),
            Action::ContactNew => self.contact_new(),
            Action::ContactDelete => self.contact_delete(),
            Action::ContactEdit => self.contact_edit_selected(),
            Action::ContactSearch => self.contact_search_start(),
            Action::ContactSearchInput(c) => self.contact_search_input(c),
            Action::ContactSearchBackspace => self.contact_search_backspace(),
            Action::ContactSearchCancel => self.contact_search_cancel(),
            // ── Contact edit form ─────────────────────────────────────
            Action::ContactEditFieldNext => self.contact_edit_field_next(),
            Action::ContactEditFieldPrev => self.contact_edit_field_prev(),
            Action::ContactEditInput(c) => self.contact_edit_input(c),
            Action::ContactEditBackspace => self.contact_edit_backspace(),
            Action::ContactEditSave => self.contact_edit_save(),
            Action::ContactEditCancel => self.contact_edit_cancel(),
            Action::ContactEditActivate => self.contact_edit_activate(),

            // ── Identity list ─────────────────────────────────────────
            Action::OpenIdentities => self.open_identities(),
            Action::IdentityNew => self.identity_new(),
            Action::IdentityEditSelected => self.identity_edit_selected(),
            Action::IdentityDelete => self.identity_delete(),
            Action::IdentitySetDefault => self.identity_set_default(),
            Action::IdentityListUp => self.identity_list_move(-1),
            Action::IdentityListDown => self.identity_list_move(1),
            Action::IdentityListClose => self.identity_list_close(),
            // ── Identity edit form ────────────────────────────────────
            Action::IdentityEditFieldNext => self.identity_edit_field_next(),
            Action::IdentityEditFieldPrev => self.identity_edit_field_prev(),
            Action::IdentityEditInput(c) => self.identity_edit_input(c),
            Action::IdentityEditBackspace => self.identity_edit_backspace(),
            Action::IdentityEditToggle => self.identity_edit_toggle(),
            Action::IdentityEditSave => self.identity_edit_save(),
            Action::IdentityEditCancel => self.identity_edit_cancel(),

            Action::None => {}
        }
    }

    // ── Action handlers ─────────────────────────────────────────────

    fn go_back(&mut self) {
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

    fn move_selection(&mut self, delta: i32) {
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

    fn jump_top(&mut self) {
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

    fn jump_bottom(&mut self) {
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

    fn page_up(&mut self) {
        if self.view == View::EnvelopeList && self.page > 1 {
            self.page -= 1;
            self.load_envelopes();
        }
    }

    fn page_down(&mut self) {
        if self.view == View::EnvelopeList && self.envelopes.len() >= PAGE_SIZE {
            self.page += 1;
            self.load_envelopes();
        }
    }

    fn select_item(&mut self) {
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

    fn scroll(&mut self, delta: i32) {
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
    fn load_identities_into(&self, cs: &mut ComposeState) {
        let Some(ref conn) = self.db else { return };
        let Some(ref account) = cs.account else {
            return;
        };
        let identities = crate::identities::list_for_account(conn, account).unwrap_or_default();
        // Pre-select the default identity, if any.
        let default_idx = identities.iter().position(|i| i.is_default);
        cs.from_identities = identities;
        cs.from_idx = default_idx;
    }

    fn compose(&mut self) {
        let mut cs = ComposeState::new(ComposeMode::New, self.acct_owned());
        self.load_identities_into(&mut cs);
        self.compose_state = Some(cs);
        self.loading = true;
        self.worker.fetch_template_write(self.acct_owned());
    }

    fn reply(&mut self, all: bool) {
        if let Some(id) = self.selected_envelope_id().map(|s| s.to_string()) {
            let mode = if all {
                ComposeMode::ReplyAll
            } else {
                ComposeMode::Reply
            };
            let mut cs = ComposeState::new(mode, self.acct_owned());
            cs.reply_to_id = Some(id.clone());
            cs.reply_to_folder = Some(self.current_folder.clone());
            self.load_identities_into(&mut cs);
            self.compose_state = Some(cs);
            self.loading = true;
            self.worker.fetch_template_reply(
                self.acct_owned(),
                self.current_folder.clone(),
                id,
                all,
            );
        }
    }

    fn forward(&mut self) {
        if let Some(id) = self.selected_envelope_id().map(|s| s.to_string()) {
            let mut cs = ComposeState::new(ComposeMode::Forward, self.acct_owned());
            cs.reply_to_id = Some(id.clone());
            cs.reply_to_folder = Some(self.current_folder.clone());
            self.load_identities_into(&mut cs);
            self.compose_state = Some(cs);
            self.loading = true;
            self.worker
                .fetch_template_forward(self.acct_owned(), self.current_folder.clone(), id);
        }
    }

    /// Activate the focused compose control.
    fn compose_enter_insert(&mut self) {
        // Grab focused + confirm_discard without keeping a borrow on self.
        let (focused, confirm_discard) = match self.compose_state.as_ref() {
            Some(cs) => (cs.focused, cs.confirm_discard),
            None => return,
        };
        if confirm_discard {
            return;
        }
        match focused {
            FocusedField::From => {
                if let Some(ref mut cs) = self.compose_state {
                    cs.cycle_from_next();
                }
            }
            FocusedField::To | FocusedField::Cc | FocusedField::Bcc | FocusedField::Subject => {
                if let Some(ref mut cs) = self.compose_state {
                    cs.focused = cs.focused.next();
                }
            }
            FocusedField::Body => {}
            FocusedField::Send => {
                self.compose_send();
            }
            FocusedField::Draft | FocusedField::Attach => {
                // Disabled — no-op
            }
            FocusedField::Discard => {
                self.compose_discard();
            }
        }
    }

    /// Exit the focused compose control back to the body.
    fn compose_exit_to_nav(&mut self) {
        let Some(ref mut cs) = self.compose_state else {
            return;
        };
        // Handle confirm-discard overlay
        if cs.confirm_discard {
            cs.confirm_discard = false;
            return;
        }
        match cs.focused {
            FocusedField::Send
            | FocusedField::Draft
            | FocusedField::Attach
            | FocusedField::Discard => {
                cs.focused = FocusedField::Body;
            }
            _ => {}
        }
    }

    fn compose_send(&mut self) {
        if let Some(ref cs) = self.compose_state {
            let template = crate::compose::reassemble_template(cs);
            self.loading = true;
            self.worker.send_template(self.acct_owned(), template);
        }
    }

    fn compose_discard(&mut self) {
        if let Some(ref cs) = self.compose_state {
            if cs.dirty || !crate::compose::body_is_empty(cs) {
                // Ask for confirmation
                if let Some(ref mut cs) = self.compose_state {
                    cs.confirm_discard = true;
                }
            } else {
                // Empty / pristine — discard immediately
                self.compose_state = None;
                self.view = View::EnvelopeList;
            }
        }
    }

    fn delete(&mut self) {
        if let Some(id) = self.selected_envelope_id().map(|s| s.to_string()) {
            self.loading = true;
            self.pending_return_to_list = self.view == View::MessageView;
            self.pending_refresh_after_action = true;
            self.worker
                .delete_message(self.acct_owned(), self.current_folder.clone(), id);
        }
    }

    fn toggle_flag(&mut self) {
        if let Some(id) = self.selected_envelope_id().map(|s| s.to_string()) {
            let is_flagged = self
                .selected_envelope()
                .map(|e| e.is_flagged())
                .unwrap_or(false);
            self.loading = true;
            self.pending_refresh_after_action = true;
            if is_flagged {
                self.worker.flag_remove(
                    self.acct_owned(),
                    self.current_folder.clone(),
                    id,
                    "flagged".to_string(),
                );
            } else {
                self.worker.flag_add(
                    self.acct_owned(),
                    self.current_folder.clone(),
                    id,
                    "flagged".to_string(),
                );
            }
        }
    }

    fn download_attachments(&mut self) {
        if let Some(id) = self.selected_envelope_id().map(|s| s.to_string()) {
            self.loading = true;
            self.worker
                .download_attachments(self.acct_owned(), self.current_folder.clone(), id);
        }
    }

    fn toggle_thread(&mut self) {
        self.threaded = !self.threaded;
        self.page = 1;
        self.load_envelopes();
        if self.threaded {
            self.set_status("Threaded view.");
        } else {
            self.set_status("Flat view.");
        }
    }

    fn enter_search(&mut self) {
        self.search_query.clear();
        self.view = View::Search;
    }

    fn submit_search(&mut self) {
        let query = self.search_query.clone();
        self.active_query = if query.is_empty() { None } else { Some(query) };
        self.page = 1;
        self.view = View::EnvelopeList;
        self.load_envelopes();
    }

    fn cancel_search(&mut self) {
        self.view = View::EnvelopeList;
    }

    fn refresh(&mut self) {
        self.ticks_since_refresh = 0;
        self.new_mail_count = 0;
        self.load_folders();
        // load_envelopes will be chained after folders complete
    }

    fn enter_account_picker(&mut self) {
        self.load_accounts();
        self.view = View::AccountList;
    }

    fn toggle_help(&mut self) {
        if self.view == View::Help {
            self.view = self.previous_view.unwrap_or(View::EnvelopeList);
            self.previous_view = None;
        } else {
            self.previous_view = Some(self.view);
            self.help_scroll = 0;
            self.view = View::Help;
        }
    }

    fn enter_move_prompt(&mut self) {
        if self.selected_envelope_id().is_some() {
            self.move_target.clear();
            self.view = View::MovePrompt;
        }
    }

    fn submit_move(&mut self) {
        let target = self.move_target.clone();
        if target.is_empty() {
            self.set_status("No target folder specified.");
            self.view = View::EnvelopeList;
            return;
        }
        if let Some(id) = self.selected_envelope_id().map(|s| s.to_string()) {
            self.loading = true;
            self.pending_refresh_after_action = true;
            self.view = View::EnvelopeList;
            self.worker
                .move_message(self.acct_owned(), self.current_folder.clone(), target, id);
        }
    }

    fn cancel_move(&mut self) {
        self.view = View::EnvelopeList;
    }

    // ── Editor key forwarding ────────────────────────────────────────

    /// Handle focused compose field input.
    fn handle_editor_key(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::KeyCode;

        // Clear send error on any key.
        if let Some(ref mut cs) = self.compose_state {
            cs.send_error = None;
        }
        let Some(ref mut cs) = self.compose_state else {
            return;
        };

        if cs.focused == FocusedField::From {
            match key.code {
                KeyCode::Char(' ') | KeyCode::Right | KeyCode::Enter => {
                    cs.cycle_from_next();
                }
                KeyCode::Left => {
                    cs.cycle_from_prev();
                }
                _ => {}
            }
            return;
        }

        if cs.autocomplete.is_some() && cs.is_header_focused() {
            match key.code {
                KeyCode::Down => {
                    if let Some(ref mut ac) = cs.autocomplete {
                        ac.move_down();
                    }
                    return;
                }
                KeyCode::Up => {
                    if let Some(ref mut ac) = cs.autocomplete {
                        ac.move_up();
                    }
                    return;
                }
                KeyCode::Enter | KeyCode::Tab => {
                    // Accept selected suggestion
                    let accepted = cs.autocomplete.as_ref().and_then(|ac| ac.current());
                    if let Some(suggestion) = accepted {
                        if let Some(f) = cs.focused_line_field_mut() {
                            let prefix = f
                                .rfind(',')
                                .map(|i| f[..=i].to_string() + " ")
                                .unwrap_or_default();
                            *f = format!("{}{}", prefix, suggestion);
                            cs.dirty = true;
                        }
                    }
                    cs.autocomplete = None;
                    if key.code == KeyCode::Tab {
                        cs.focused = cs.focused.next();
                    }
                    return;
                }
                KeyCode::Esc => {
                    cs.autocomplete = None;
                    return;
                }
                _ => {} // fall through
            }
        }

        if matches!(
            cs.focused,
            FocusedField::To | FocusedField::Cc | FocusedField::Bcc | FocusedField::Subject
        ) {
            let is_address = matches!(
                cs.focused,
                FocusedField::To | FocusedField::Cc | FocusedField::Bcc
            );
            let mut modified = false;
            let mut move_focus = false;
            if let Some(field) = cs.focused_line_field_mut() {
                match key.code {
                    KeyCode::Char(c)
                        if !key.modifiers.intersects(
                            crossterm::event::KeyModifiers::CONTROL
                                | crossterm::event::KeyModifiers::ALT,
                        ) =>
                    {
                        field.push(c);
                        modified = true;
                    }
                    KeyCode::Backspace => {
                        modified = field.pop().is_some();
                    }
                    KeyCode::Enter => {
                        move_focus = true;
                    }
                    _ => {}
                }
            }
            if modified {
                cs.dirty = true;
            }
            if move_focus {
                cs.autocomplete = None;
                cs.focused = cs.focused.next();
            }
            let _ = cs;
            if is_address && modified {
                self.update_autocomplete();
            }
            return;
        }

        if cs.focused == FocusedField::Body {
            let result = cs.body.handle_key(key);
            if result.text_modified {
                cs.dirty = true;
            }
        }
    }

    // ── Autocomplete ─────────────────────────────────────────────────

    /// Update the autocomplete popup for the currently focused address field.
    /// Called after every character typed in To/Cc/Bcc.  Results come from
    /// a synchronous search of the encrypted contacts DB.
    fn update_autocomplete(&mut self) {
        // Extract the current query token (text after the last comma).
        let query = {
            let Some(ref cs) = self.compose_state else {
                return;
            };
            let field_value = match cs.focused {
                FocusedField::To => &cs.to,
                FocusedField::Cc => &cs.cc,
                FocusedField::Bcc => &cs.bcc,
                _ => return,
            };
            // Get the last token after a comma (in case of multiple addresses)
            let token = field_value
                .rfind(',')
                .map(|i| field_value[i + 1..].trim())
                .unwrap_or(field_value.as_str());
            token.to_string()
        };

        // Require at least 2 characters to trigger autocomplete.
        if query.len() < 2 {
            if let Some(ref mut cs) = self.compose_state {
                cs.autocomplete = None;
            }
            return;
        }

        // Run search synchronously against the DB.
        let results: Vec<(Option<String>, String)> = if let Some(ref conn) = self.db {
            crate::contacts::search(conn, &query, 8)
                .unwrap_or_default()
                .into_iter()
                .map(|c| (c.name, c.email))
                .collect()
        } else {
            vec![]
        };

        if let Some(ref mut cs) = self.compose_state {
            if results.is_empty() {
                cs.autocomplete = None;
            } else {
                let field = cs.focused;
                cs.autocomplete = Some(crate::compose::AutocompleteState::new(field, results));
            }
        }
    }

    // ── Contacts ─────────────────────────────────────────────────────

    fn open_contacts(&mut self) {
        // Load contacts from DB if available.
        if let Some(ref conn) = self.db {
            match crate::contacts::list(conn, None) {
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

    fn contact_search_start(&mut self) {
        self.contact_search.clear();
        self.contact_search_active = true;
        self.view = keys::View::ContactSearch;
    }

    fn contact_search_input(&mut self, c: char) {
        self.contact_search.push(c);
        self.refresh_contact_search();
    }

    fn contact_search_backspace(&mut self) {
        self.contact_search.pop();
        self.refresh_contact_search();
    }

    fn contact_search_cancel(&mut self) {
        self.contact_search_active = false;
        self.view = keys::View::Contacts;
        // If search is cleared, reload full list; otherwise keep current results.
        if self.contact_search.is_empty() {
            if let Some(ref conn) = self.db {
                if let Ok(contacts) = crate::contacts::list(conn, None) {
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
    fn refresh_contact_search(&mut self) {
        if self.contact_search.is_empty() {
            // Show all contacts when query is empty
            if let Some(ref conn) = self.db {
                if let Ok(contacts) = crate::contacts::list(conn, None) {
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

    fn contact_new(&mut self) {
        self.contact_edit_state = Some(ContactEditState::new());
        self.previous_view = Some(self.view);
        self.view = keys::View::ContactEdit;
    }

    fn contact_edit_selected(&mut self) {
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

    fn contact_edit_field_next(&mut self) {
        if let Some(ref mut s) = self.contact_edit_state {
            s.focused = s.focused.next();
        }
    }

    fn contact_edit_field_prev(&mut self) {
        if let Some(ref mut s) = self.contact_edit_state {
            s.focused = s.focused.prev();
        }
    }

    fn contact_edit_input(&mut self, c: char) {
        if let Some(ref mut s) = self.contact_edit_state {
            if let Some(field) = s.focused_field_mut() {
                field.push(c);
            }
            s.error = None;
        }
    }

    fn contact_edit_backspace(&mut self) {
        if let Some(ref mut s) = self.contact_edit_state {
            if let Some(field) = s.focused_field_mut() {
                field.pop();
            }
        }
    }

    /// Enter key in contact edit: activate action buttons or advance field.
    fn contact_edit_activate(&mut self) {
        let focused = self.contact_edit_state.as_ref().map(|s| s.focused);
        match focused {
            Some(crate::contact_edit::ContactField::Save) => self.contact_edit_save(),
            Some(crate::contact_edit::ContactField::Cancel) => self.contact_edit_cancel(),
            Some(_) => self.contact_edit_field_next(),
            None => {}
        }
    }

    fn contact_edit_save(&mut self) {
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
                                if let Ok(contacts) = crate::contacts::list(conn, None) {
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

    fn contact_edit_cancel(&mut self) {
        self.contact_edit_state = None;
        let prev = self.previous_view.unwrap_or(keys::View::Contacts);
        self.view = prev;
        self.previous_view = None;
    }

    fn contact_delete(&mut self) {
        let Some(idx) = self.contact_index else {
            return;
        };
        let Some(contact) = self.contacts.get(idx) else {
            return;
        };
        let id = contact.id;

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

    fn reload_identities(&mut self) {
        if let Some(ref conn) = self.db {
            let account = self.account_name.as_deref().unwrap_or("").to_string();
            self.identities =
                crate::identities::list_for_account(conn, &account).unwrap_or_default();
            if self.identities.is_empty() {
                self.identity_index = None;
            } else {
                let idx = self
                    .identity_index
                    .unwrap_or(0)
                    .min(self.identities.len() - 1);
                self.identity_index = Some(idx);
            }
        }
    }

    fn open_identities(&mut self) {
        self.reload_identities();
        self.previous_view = Some(self.view);
        self.view = View::IdentityList;
    }

    fn identity_list_move(&mut self, delta: i32) {
        let len = self.identities.len();
        if len == 0 {
            return;
        }
        let current = self.identity_index.unwrap_or(0);
        let next = if delta > 0 {
            (current + 1).min(len - 1)
        } else {
            current.saturating_sub(1)
        };
        self.identity_index = Some(next);
    }

    fn identity_list_close(&mut self) {
        self.view = self.previous_view.unwrap_or(View::EnvelopeList);
        self.previous_view = None;
    }

    fn identity_new(&mut self) {
        let account = self.account_name.as_deref().unwrap_or("").to_string();
        self.identity_edit_state = Some(IdentityEditState::new(&account));
        self.view = View::IdentityEdit;
    }

    fn identity_edit_selected(&mut self) {
        let identity = self
            .identity_index
            .and_then(|i| self.identities.get(i))
            .cloned();
        if let Some(id) = identity {
            self.identity_edit_state = Some(IdentityEditState::from_identity(&id));
            self.view = View::IdentityEdit;
        }
    }

    fn identity_delete(&mut self) {
        let Some(idx) = self.identity_index else {
            return;
        };
        let Some(identity) = self.identities.get(idx) else {
            return;
        };
        let id = identity.id;
        if let Some(ref conn) = self.db {
            match crate::identities::delete(conn, id) {
                Ok(()) => {
                    self.set_status("Identity deleted.");
                    self.reload_identities();
                }
                Err(e) => self.set_error(&format!("delete identity: {e}")),
            }
        }
    }

    fn identity_set_default(&mut self) {
        let Some(idx) = self.identity_index else {
            return;
        };
        let Some(identity) = self.identities.get(idx) else {
            return;
        };
        let id = identity.id;
        let account = identity.account.clone();
        if let Some(ref conn) = self.db {
            match crate::identities::set_default(conn, &account, id) {
                Ok(()) => {
                    self.set_status("Default identity set.");
                    self.reload_identities();
                }
                Err(e) => self.set_error(&format!("set default: {e}")),
            }
        }
    }

    fn identity_edit_field_next(&mut self) {
        if let Some(ref mut s) = self.identity_edit_state {
            s.focused = s.focused.next();
        }
    }

    fn identity_edit_field_prev(&mut self) {
        if let Some(ref mut s) = self.identity_edit_state {
            s.focused = s.focused.prev();
        }
    }

    fn identity_edit_input(&mut self, c: char) {
        if let Some(ref mut s) = self.identity_edit_state {
            if let Some(field) = s.focused_field_mut() {
                field.push(c);
            }
            s.error = None;
        }
    }

    fn identity_edit_backspace(&mut self) {
        if let Some(ref mut s) = self.identity_edit_state {
            if let Some(field) = s.focused_field_mut() {
                field.pop();
            }
        }
    }

    fn identity_edit_toggle(&mut self) {
        use crate::identity_edit::IdentityField;
        let focused = self.identity_edit_state.as_ref().map(|s| s.focused);
        match focused {
            Some(IdentityField::Save) => self.identity_edit_save(),
            Some(IdentityField::Cancel) => self.identity_edit_cancel(),
            Some(IdentityField::IsDefault) => {
                if let Some(ref mut s) = self.identity_edit_state {
                    s.toggle_default();
                }
            }
            Some(_) => {
                // Enter on a text field advances to the next field.
                if let Some(ref mut s) = self.identity_edit_state {
                    s.focused = s.focused.next();
                }
            }
            None => {}
        }
    }

    fn identity_edit_save(&mut self) {
        // Validate and extract data while borrowing immutably.
        let validation = self
            .identity_edit_state
            .as_ref()
            .map(|s| (s.validate(), s.identity_id, s.account.clone()));

        let Some((validation_result, identity_id, account)) = validation else {
            return;
        };

        match validation_result {
            Ok((name, display_name, email, is_default)) => {
                if let Some(ref conn) = self.db {
                    let result = if let Some(id) = identity_id {
                        crate::identities::delete(conn, id).and_then(|_| {
                            crate::identities::add(
                                conn,
                                &account,
                                name.as_deref(),
                                display_name.as_deref(),
                                &email,
                                is_default,
                            )
                            .map(|_| ())
                        })
                    } else {
                        crate::identities::add(
                            conn,
                            &account,
                            name.as_deref(),
                            display_name.as_deref(),
                            &email,
                            is_default,
                        )
                        .map(|_| ())
                    };
                    match result {
                        Ok(()) => {
                            self.identity_edit_state = None;
                            self.view = View::IdentityList;
                            self.set_status("Identity saved.");
                            self.reload_identities();
                        }
                        Err(e) => {
                            if let Some(ref mut s) = self.identity_edit_state {
                                s.error = Some(format!("Save failed: {e}"));
                            }
                        }
                    }
                }
            }
            Err(msg) => {
                if let Some(ref mut s) = self.identity_edit_state {
                    s.error = Some(msg);
                }
            }
        }
    }

    fn identity_edit_cancel(&mut self) {
        self.identity_edit_state = None;
        self.view = View::IdentityList;
    }
}

#[cfg(test)]
mod tests {
    use crate::himalaya::diagnostics as himalaya_diagnostics;

    #[test]
    fn explain_himalaya_error_flags_maildir_as_non_auth() {
        let raw = "himalaya error: cannot open local maildir";
        let explained = himalaya_diagnostics::explain(Some("maildir"), raw);
        assert!(explained.contains("not an authentication error"));
    }

    #[test]
    fn explain_himalaya_error_flags_keyring_failures() {
        let raw = "himalaya error: failed to talk to Secret Service keyring";
        let explained = himalaya_diagnostics::explain(Some("imap"), raw);
        assert!(explained.contains("Keyring secret missing or inaccessible"));
    }

    #[test]
    fn explain_himalaya_error_flags_oauth_failures() {
        let raw = "himalaya error: invalid_grant while refreshing oauth token";
        let explained = himalaya_diagnostics::explain(Some("imap"), raw);
        assert!(explained.contains("OAuth credentials need reconfiguration"));
    }
}
