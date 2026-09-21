/* Compose editor state: template parsing, reassembly, and field management.
The compose flow:
1. Fetch a backend-owned draft template for write/reply/forward
2. Parse into header fields + body
3. Edit in the TUI (compose editor for body, single-line inputs for headers)
4. Reassemble into a template string
5. Send via the active mail service backend */

use crate::compose_editor::ComposeEditor;
use crate::identities::Identity;
use crate::keys::EditMode;

/// Which mode we're in: new message, reply, reply-all, or forward.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComposeMode {
    New,
    Reply,
    ReplyAll,
    Forward,
}

/// Which field / action-bar button currently has focus.
///
/// Tab cycle order:
///   From → To → Cc → Bcc → Subject → Body → Send → Draft → Attach → Discard → From
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedField {
    From,
    To,
    Cc,
    Bcc,
    Subject,
    Body,
    // ── Action bar buttons ──────────────────────────────────────────
    Send,
    Draft,
    Attach,
    Discard,
}

impl FocusedField {
    pub fn next(self) -> Self {
        match self {
            FocusedField::From => FocusedField::To,
            FocusedField::To => FocusedField::Cc,
            FocusedField::Cc => FocusedField::Bcc,
            FocusedField::Bcc => FocusedField::Subject,
            FocusedField::Subject => FocusedField::Body,
            FocusedField::Body => FocusedField::Send,
            FocusedField::Send => FocusedField::Draft,
            FocusedField::Draft => FocusedField::Attach,
            FocusedField::Attach => FocusedField::Discard,
            FocusedField::Discard => FocusedField::From,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            FocusedField::From => FocusedField::Discard,
            FocusedField::To => FocusedField::From,
            FocusedField::Cc => FocusedField::To,
            FocusedField::Bcc => FocusedField::Cc,
            FocusedField::Subject => FocusedField::Bcc,
            FocusedField::Body => FocusedField::Subject,
            FocusedField::Send => FocusedField::Body,
            FocusedField::Draft => FocusedField::Send,
            FocusedField::Attach => FocusedField::Draft,
            FocusedField::Discard => FocusedField::Attach,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            FocusedField::From => "From",
            FocusedField::To => "To",
            FocusedField::Cc => "Cc",
            FocusedField::Bcc => "Bcc",
            FocusedField::Subject => "Subject",
            FocusedField::Body => "Body",
            FocusedField::Send => "Send",
            FocusedField::Draft => "Draft",
            FocusedField::Attach => "Attach",
            FocusedField::Discard => "Discard",
        }
    }

    /// True if this is an action-bar button (not a text/content field).
    pub fn is_action_button(self) -> bool {
        matches!(
            self,
            FocusedField::Send | FocusedField::Draft | FocusedField::Attach | FocusedField::Discard
        )
    }
}

/// Autocomplete suggestion popup state.
#[derive(Debug, Clone)]
pub struct AutocompleteState {
    /// The suggestions (name, email).
    pub suggestions: Vec<(Option<String>, String)>,
    /// Currently selected index.
    pub selected: usize,
    /// The field that triggered autocomplete.
    pub field: FocusedField,
}

impl AutocompleteState {
    pub fn new(field: FocusedField, suggestions: Vec<(Option<String>, String)>) -> Self {
        Self {
            suggestions,
            selected: 0,
            field,
        }
    }

    pub fn move_up(&mut self) {
        if self.suggestions.is_empty() {
            return;
        }
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn move_down(&mut self) {
        if self.suggestions.is_empty() {
            return;
        }
        self.selected = (self.selected + 1).min(self.suggestions.len() - 1);
    }

    /// Formatted display for the currently selected suggestion.
    pub fn current(&self) -> Option<String> {
        self.suggestions
            .get(self.selected)
            .map(|(name, email)| match name {
                Some(n) if !n.is_empty() => format!("\"{}\" <{}>", n, email),
                _ => email.clone(),
            })
    }
}

/// Full state of the compose editor.
pub struct ComposeState {
    pub mode: ComposeMode,
    /// Account name used by the active mail service.
    pub account: Option<String>,
    /// Available sender identities for this account (loaded from DB).
    pub from_identities: Vec<Identity>,
    /// Currently selected identity index into `from_identities`.
    /// `None` means "use account default" (no explicit From header).
    pub from_idx: Option<usize>,
    /// To: header value (raw text, comma-separated addresses)
    pub to: String,
    pub cc: String,
    pub bcc: String,
    pub subject: String,
    /// Threading headers carried from a reply/forward template (RFC 5322).
    pub in_reply_to: Option<String>,
    pub references: Option<String>,
    /// Compose editor state for the message body.
    pub body: ComposeEditor,
    /// Which field has keyboard focus
    pub focused: FocusedField,
    /// Autocomplete popup, if active
    pub autocomplete: Option<AutocompleteState>,
    /// For reply/forward: the source message ID
    pub reply_to_id: Option<String>,
    pub reply_to_folder: Option<String>,
    /// Whether we're in the discard-confirm state
    pub confirm_discard: bool,
    /// Whether the body has been modified
    pub dirty: bool,
    /// Send error message to display
    pub send_error: Option<String>,
    /// Nav/Insert modal editing mode (for header fields and action-bar nav)
    pub edit_mode: EditMode,
}

impl ComposeState {
    pub fn new(mode: ComposeMode, account: Option<String>) -> Self {
        Self {
            mode,
            account,
            from_identities: Vec::new(),
            from_idx: None,
            to: String::new(),
            cc: String::new(),
            bcc: String::new(),
            subject: String::new(),
            in_reply_to: None,
            references: None,
            body: ComposeEditor::default(),
            focused: FocusedField::From,
            autocomplete: None,
            reply_to_id: None,
            reply_to_folder: None,
            confirm_discard: false,
            dirty: false,
            send_error: None,
            edit_mode: EditMode::Nav,
        }
    }

    /// The currently selected identity, if any.
    pub fn selected_identity(&self) -> Option<&Identity> {
        self.from_idx.and_then(|i| self.from_identities.get(i))
    }

    /// Cycle to the next identity (wraps; None → 0 → 1 → … → None).
    pub fn cycle_from_next(&mut self) {
        if self.from_identities.is_empty() {
            return;
        }
        self.from_idx = match self.from_idx {
            None => Some(0),
            Some(i) if i + 1 >= self.from_identities.len() => None,
            Some(i) => Some(i + 1),
        };
    }

    /// Cycle to the previous identity.
    pub fn cycle_from_prev(&mut self) {
        if self.from_identities.is_empty() {
            return;
        }
        self.from_idx = match self.from_idx {
            None => Some(self.from_identities.len() - 1),
            Some(0) => None,
            Some(i) => Some(i - 1),
        };
    }

    /// Return the currently focused single-line field as a mutable reference.
    /// From, Body and action-bar buttons have no editable text; they return None.
    pub fn focused_line_field_mut(&mut self) -> Option<&mut String> {
        match self.focused {
            FocusedField::From => None,
            FocusedField::To => Some(&mut self.to),
            FocusedField::Cc => Some(&mut self.cc),
            FocusedField::Bcc => Some(&mut self.bcc),
            FocusedField::Subject => Some(&mut self.subject),
            FocusedField::Body => None,
            FocusedField::Send
            | FocusedField::Draft
            | FocusedField::Attach
            | FocusedField::Discard => None,
        }
    }

    /// True if focus is on a header field (not the body editor or action bar).
    pub fn is_header_focused(&self) -> bool {
        matches!(
            self.focused,
            FocusedField::From
                | FocusedField::To
                | FocusedField::Cc
                | FocusedField::Bcc
                | FocusedField::Subject
        )
    }
}
