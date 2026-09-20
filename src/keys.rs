use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// The current view determines which keybindings are active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::enum_variant_names)]
pub enum View {
    EnvelopeList,
    MessageView,
    FolderList,
    AccountList,
    Search,
    Help,
    MovePrompt,
    /// Native compose / reply / forward editor.
    Compose,
    /// Address book browser.
    Contacts,
    /// Contact search input mode (within the address book).
    ContactSearch,
    /// Contact add/edit form.
    ContactEdit,
    /// Identity list for the current account.
    IdentityList,
    /// Identity add/edit form.
    IdentityEdit,
}

/// Editing mode for forms that still distinguish navigation vs text entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditMode {
    /// Navigation-focused controls.
    Nav,
    /// Direct text entry.
    Insert,
}

/// Coarse compose focus buckets used by contextual key resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComposeFocus {
    From,
    Header,
    Body,
    ActionBar,
}

/// Runtime compose context needed to resolve keys correctly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComposeKeyContext {
    /// Which compose region currently owns focus.
    pub focus: ComposeFocus,
    /// Nav vs Insert for form-style fields that still use it.
    pub edit_mode: EditMode,
    /// Whether the body editor currently has an active search session.
    pub body_search_active: bool,
    /// Whether contact-autocomplete suggestions are visible.
    pub autocomplete_visible: bool,
    /// Whether the discard-confirmation modal is currently shown.
    pub confirm_discard_visible: bool,
}

/// Actions the app can take in response to a key press.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Quit,
    Back,
    MoveUp,
    MoveDown,
    PageUp,
    PageDown,
    JumpTop,
    JumpBottom,
    Select,
    OpenMessage,
    Compose,
    Reply,
    ReplyAll,
    Forward,
    Delete,
    ToggleFlag,
    DownloadAttachments,
    ToggleThread,
    Search,
    SearchSubmit,
    SearchCancel,
    SearchInput(char),
    SearchBackspace,
    Refresh,
    SwitchAccount,
    ToggleHelp,
    FocusFolders,
    FocusEnvelopes,
    MoveMessage,
    MoveInput(char),
    MoveBackspace,
    MoveSubmit,
    MoveCancel,
    ScrollUp,
    ScrollDown,
    // ── Compose editor ────────────────────────────────────────────────
    ComposeFieldNext,
    ComposeFieldPrev,
    ComposeLeaveBodyNext,
    ComposeLeaveBodyPrev,
    ComposeSend,
    ComposeDiscard,
    ComposeConfirmDiscard,
    ComposeCancelDiscard,
    ComposeInput(char),
    ComposeBackspace,
    /// Activate the focused compose control.
    ComposeEnterInsert,
    /// Leave the focused compose control back to the main compose flow.
    ComposeExitToNav,
    // ── Contacts browser ──────────────────────────────────────────────
    OpenContacts,
    ContactNew,
    ContactDelete,
    ContactEdit,
    ContactSearch,
    ContactSearchInput(char),
    ContactSearchBackspace,
    ContactSearchCancel,
    // ── Contact edit form ─────────────────────────────────────────────
    ContactEditFieldNext,
    ContactEditFieldPrev,
    ContactEditInput(char),
    ContactEditBackspace,
    ContactEditSave,
    ContactEditCancel,
    /// Enter key on contact edit: activates focused action-button or advances field.
    ContactEditActivate,
    // ── Identity list ─────────────────────────────────────────────────
    OpenIdentities,
    IdentityNew,
    IdentityEditSelected,
    IdentityDelete,
    IdentitySetDefault,
    IdentityListUp,
    IdentityListDown,
    IdentityListClose,
    // ── Identity edit form ────────────────────────────────────────────
    IdentityEditFieldNext,
    IdentityEditFieldPrev,
    IdentityEditInput(char),
    IdentityEditBackspace,
    IdentityEditToggle,
    IdentityEditSave,
    IdentityEditCancel,
    // ── Passthrough for the compose editor / focused field ────────────
    /// Raw key event forwarded to the compose editor or focused field.
    EditorKey(crossterm::event::KeyEvent),
    None,
}

/// Resolve a key event into an action given the current view context.
pub fn resolve(view: View, key: KeyEvent) -> Action {
    match view {
        View::Compose => return resolve_compose(key),
        View::Contacts => return resolve_contacts(key),
        View::ContactSearch => return resolve_contact_search(key),
        View::ContactEdit => return resolve_contact_edit(key),
        View::IdentityList => return resolve_identity_list(key),
        View::IdentityEdit => return resolve_identity_edit(key),
        _ => {}
    }

    // Global keybindings (handled first)
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        return match key.code {
            KeyCode::Char('c') | KeyCode::Char('q') => Action::Quit,
            KeyCode::Char('a') => Action::SwitchAccount,
            KeyCode::Char('r') => Action::Refresh,
            KeyCode::Char('b') => Action::OpenContacts,
            _ => Action::None,
        };
    }

    match view {
        View::EnvelopeList => resolve_envelope_list(key),
        View::MessageView => resolve_message_view(key),
        View::FolderList => resolve_folder_list(key),
        View::AccountList => resolve_account_list(key),
        View::Search => resolve_search(key),
        View::Help => resolve_help(key),
        View::MovePrompt => resolve_move_prompt(key),
        View::ContactSearch => resolve_contact_search(key),
        View::ContactEdit => resolve_contact_edit(key),
        // Already handled above
        View::Compose | View::Contacts | View::IdentityList | View::IdentityEdit => Action::None,
    }
}

/// Resolve compose keys with compose-state context.
///
/// Compose is focus-driven: the shell owns modal overlays, field cycling, and
/// action-bar activation, while the focused field handles its own editing.
///
/// Priority order (highest first):
/// 1) discard-confirm modal interception
/// 2) global compose shortcuts (`Ctrl+C` / `Ctrl+Q`)
/// 3) autocomplete popup navigation/accept keys
/// 4) compose shell controls (`Tab`, `Shift+Tab`, action-bar `Enter` / `Esc`)
/// 5) passthrough to the focused compose field
pub fn resolve_compose_with_context(key: KeyEvent, ctx: ComposeKeyContext) -> Action {
    // Discard confirmation modal owns key handling while visible.
    if ctx.confirm_discard_visible {
        return match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => Action::ComposeConfirmDiscard,
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => Action::ComposeCancelDiscard,
            _ => Action::None,
        };
    }

    // Allow Ctrl+C / Ctrl+Q globally in compose as quit-discard
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        return match key.code {
            KeyCode::Char('c') | KeyCode::Char('q') => Action::ComposeDiscard,
            _ => Action::EditorKey(key),
        };
    }

    // If autocomplete popup is open, let app-level popup handler own navigation
    // and acceptance keys.
    if ctx.autocomplete_visible {
        match key.code {
            KeyCode::Down | KeyCode::Up | KeyCode::Enter | KeyCode::Tab | KeyCode::Esc => {
                return Action::EditorKey(key);
            }
            _ => {}
        }
    }

    match key.code {
        KeyCode::Tab if ctx.focus == ComposeFocus::Body && ctx.body_search_active => {
            Action::ComposeLeaveBodyNext
        }
        KeyCode::BackTab if ctx.focus == ComposeFocus::Body && ctx.body_search_active => {
            Action::ComposeLeaveBodyPrev
        }
        KeyCode::Tab => Action::ComposeFieldNext,
        KeyCode::BackTab => Action::ComposeFieldPrev,
        KeyCode::Down if ctx.focus != ComposeFocus::Body => Action::ComposeFieldNext,
        KeyCode::Up if ctx.focus != ComposeFocus::Body => Action::ComposeFieldPrev,
        KeyCode::Enter if ctx.focus == ComposeFocus::ActionBar => Action::ComposeEnterInsert,
        KeyCode::Esc if ctx.focus == ComposeFocus::ActionBar => Action::ComposeExitToNav,
        _ => Action::EditorKey(key),
    }
}

fn resolve_envelope_list(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Char('q') => Action::Quit,
        KeyCode::Char('j') | KeyCode::Down => Action::MoveDown,
        KeyCode::Char('k') | KeyCode::Up => Action::MoveUp,
        KeyCode::Char('g') => Action::JumpTop,
        KeyCode::Char('G') => Action::JumpBottom,
        KeyCode::Enter => Action::OpenMessage,
        KeyCode::Char('c') => Action::Compose,
        KeyCode::Char('d') => Action::Delete,
        KeyCode::Char('m') => Action::MoveMessage,
        KeyCode::Char('!') => Action::ToggleFlag,
        KeyCode::Char('/') => Action::Search,
        KeyCode::Char('n') => Action::PageDown,
        KeyCode::Char('p') => Action::PageUp,
        KeyCode::Char('t') => Action::ToggleThread,
        KeyCode::Char('?') => Action::ToggleHelp,
        KeyCode::Char('I') => Action::OpenIdentities,
        KeyCode::Tab => Action::FocusFolders,
        KeyCode::Esc => Action::Quit,
        _ => Action::None,
    }
}

fn resolve_message_view(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => Action::Back,
        KeyCode::Char('j') | KeyCode::Down => Action::ScrollDown,
        KeyCode::Char('k') | KeyCode::Up => Action::ScrollUp,
        KeyCode::Char(' ') => Action::PageDown,
        KeyCode::Char('r') => Action::Reply,
        KeyCode::Char('R') => Action::ReplyAll,
        KeyCode::Char('f') => Action::Forward,
        KeyCode::Char('d') => Action::Delete,
        KeyCode::Char('a') => Action::DownloadAttachments,
        KeyCode::Char('?') => Action::ToggleHelp,
        KeyCode::Char('g') => Action::JumpTop,
        KeyCode::Char('G') => Action::JumpBottom,
        _ => Action::None,
    }
}

fn resolve_folder_list(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => Action::FocusEnvelopes,
        KeyCode::Char('j') | KeyCode::Down => Action::MoveDown,
        KeyCode::Char('k') | KeyCode::Up => Action::MoveUp,
        KeyCode::Enter => Action::Select,
        KeyCode::Tab => Action::FocusEnvelopes,
        KeyCode::Char('?') => Action::ToggleHelp,
        KeyCode::Char('I') => Action::OpenIdentities,
        _ => Action::None,
    }
}

fn resolve_account_list(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Esc => Action::Back,
        KeyCode::Char('q') => Action::Back,
        KeyCode::Char('j') | KeyCode::Down => Action::MoveDown,
        KeyCode::Char('k') | KeyCode::Up => Action::MoveUp,
        KeyCode::Enter => Action::Select,
        _ => Action::None,
    }
}

fn resolve_search(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Enter => Action::SearchSubmit,
        KeyCode::Esc => Action::SearchCancel,
        KeyCode::Backspace => Action::SearchBackspace,
        KeyCode::Char(c) => Action::SearchInput(c),
        _ => Action::None,
    }
}

fn resolve_help(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') => Action::ToggleHelp,
        KeyCode::Char('j') | KeyCode::Down => Action::ScrollDown,
        KeyCode::Char('k') | KeyCode::Up => Action::ScrollUp,
        _ => Action::None,
    }
}

fn resolve_move_prompt(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Enter => Action::MoveSubmit,
        KeyCode::Esc => Action::MoveCancel,
        KeyCode::Backspace => Action::MoveBackspace,
        KeyCode::Char(c) => Action::MoveInput(c),
        _ => Action::None,
    }
}

/// Backward-compatible compose resolver used by tests and any call sites that
/// do not have live compose state available.
///
/// Uses a default "Header + Nav + no popups" context.
fn resolve_compose(key: KeyEvent) -> Action {
    resolve_compose_with_context(
        key,
        ComposeKeyContext {
            focus: ComposeFocus::Header,
            edit_mode: EditMode::Nav,
            body_search_active: false,
            autocomplete_visible: false,
            confirm_discard_visible: false,
        },
    )
}

fn resolve_contact_edit(key: KeyEvent) -> Action {
    // Ctrl+C / Ctrl+Q cancel without saving
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        return match key.code {
            KeyCode::Char('c') | KeyCode::Char('q') => Action::ContactEditCancel,
            _ => Action::None,
        };
    }
    match key.code {
        KeyCode::Esc => Action::ContactEditCancel,
        KeyCode::Tab => Action::ContactEditFieldNext,
        KeyCode::BackTab => Action::ContactEditFieldPrev,
        KeyCode::Enter => Action::ContactEditActivate,
        KeyCode::Backspace => Action::ContactEditBackspace,
        KeyCode::Char(c) => Action::ContactEditInput(c),
        _ => Action::None,
    }
}

fn resolve_contact_search(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Esc => Action::ContactSearchCancel,
        KeyCode::Enter => Action::ContactSearchCancel,
        KeyCode::Backspace => Action::ContactSearchBackspace,
        KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            Action::ContactSearchInput(c)
        }
        _ => Action::None,
    }
}

fn resolve_identity_list(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => Action::IdentityListClose,
        KeyCode::Char('j') | KeyCode::Down => Action::IdentityListDown,
        KeyCode::Char('k') | KeyCode::Up => Action::IdentityListUp,
        KeyCode::Char('n') => Action::IdentityNew,
        KeyCode::Char('e') | KeyCode::Enter => Action::IdentityEditSelected,
        KeyCode::Char('d') => Action::IdentityDelete,
        KeyCode::Char('s') => Action::IdentitySetDefault,
        _ => Action::None,
    }
}

fn resolve_identity_edit(key: KeyEvent) -> Action {
    // Ctrl+C / Ctrl+Q cancel without saving
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        return match key.code {
            KeyCode::Char('c') | KeyCode::Char('q') => Action::IdentityEditCancel,
            _ => Action::None,
        };
    }
    match key.code {
        KeyCode::Esc => Action::IdentityEditCancel,
        KeyCode::Tab => Action::IdentityEditFieldNext,
        KeyCode::BackTab => Action::IdentityEditFieldPrev,
        KeyCode::Enter => Action::IdentityEditToggle,
        KeyCode::Backspace => Action::IdentityEditBackspace,
        KeyCode::Char(c) => Action::IdentityEditInput(c),
        _ => Action::None,
    }
}

fn resolve_contacts(key: KeyEvent) -> Action {
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        return match key.code {
            KeyCode::Char('b') => Action::Back,
            KeyCode::Char('c') | KeyCode::Char('q') => Action::Quit,
            _ => Action::None,
        };
    }
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => Action::Back,
        KeyCode::Char('j') | KeyCode::Down => Action::MoveDown,
        KeyCode::Char('k') | KeyCode::Up => Action::MoveUp,
        KeyCode::Char('g') => Action::JumpTop,
        KeyCode::Char('G') => Action::JumpBottom,
        KeyCode::Char('n') => Action::ContactNew,
        KeyCode::Char('d') => Action::ContactDelete,
        KeyCode::Char('e') => Action::ContactEdit,
        KeyCode::Char('/') => Action::ContactSearch,
        KeyCode::Char('c') => Action::Compose,
        _ => Action::None,
    }
}

/// Keybinding hint strings for the status bar, per view.
pub fn hints(view: View) -> Vec<(&'static str, &'static str)> {
    match view {
        View::EnvelopeList => vec![
            ("j/k", "nav"),
            ("Enter", "read"),
            ("c", "compose"),
            ("d", "del"),
            ("m", "move"),
            ("!", "flag"),
            ("t", "thread"),
            ("/", "search"),
            ("Tab", "folders"),
            ("Ctrl+b", "contacts"),
            ("?", "help"),
        ],
        View::MessageView => vec![
            ("j/k", "scroll"),
            ("r", "reply"),
            ("R", "all"),
            ("f", "fwd"),
            ("d", "del"),
            ("a", "attach"),
            ("q", "back"),
            ("?", "help"),
        ],
        View::FolderList => vec![
            ("j/k", "nav"),
            ("Enter", "select"),
            ("Tab", "emails"),
            ("?", "help"),
        ],
        View::AccountList => vec![("j/k", "nav"), ("Enter", "select"), ("Esc", "cancel")],
        View::Search => vec![("Enter", "search"), ("Esc", "cancel")],
        View::Help => vec![("j/k", "scroll"), ("q/?/Esc", "close")],
        View::MovePrompt => vec![("Enter", "move"), ("Esc", "cancel")],
        View::Compose => vec![
            ("Tab/j/k", "nav"),
            ("Enter", "insert"),
            ("Esc", "nav/discard"),
        ],
        View::Contacts => vec![
            ("j/k", "nav"),
            ("n", "new"),
            ("e", "edit"),
            ("d", "del"),
            ("/", "search"),
            ("q", "close"),
        ],
        View::ContactSearch => vec![("Enter", "confirm"), ("Esc", "cancel")],
        View::ContactEdit => vec![
            ("Tab/Enter", "next field"),
            ("Tab→Save→Enter", "save"),
            ("Esc", "cancel"),
        ],
        View::IdentityList => vec![
            ("j/k", "nav"),
            ("n", "new"),
            ("e", "edit"),
            ("d", "delete"),
            ("s", "set default"),
            ("q/Esc", "close"),
        ],
        View::IdentityEdit => vec![
            ("Tab/Enter", "next field"),
            ("Space", "toggle"),
            ("Tab→Save→Enter", "save"),
            ("Esc", "cancel"),
        ],
    }
}
