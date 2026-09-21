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
    ToggleRead,
    SyncFolder,
    MarkFolderRead,
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
