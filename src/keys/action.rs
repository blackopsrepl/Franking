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
    /// Passphrase prompt for unlocking PGP secret keys.
    PassphrasePrompt,
    /// Attachment list overlay for the current message.
    AttachmentList,
    /// Folder management prompt (create, rename, delete).
    FolderPrompt,
    /// In-message search prompt.
    MessageSearch,
    /// Server-side Sieve script browser.
    SieveScripts,
    /// New-script name prompt.
    SieveName,
    /// Sieve script editor.
    SieveEdit,
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
    /// Show every raw header in the message view.
    ToggleHeaders,
    /// Save the loaded message as an .eml file.
    SaveMessage,
    FocusFolders,
    FocusEnvelopes,
    MoveMessage,
    MoveInput(char),
    MoveBackspace,
    MoveNext,
    MovePrev,
    MoveSubmit,
    MoveCancel,
    // ── Crypto unlock prompt ──────────────────────────────────────────
    UnlockPrompt,
    UnlockInput(char),
    UnlockBackspace,
    UnlockSubmit,
    UnlockCancel,
    /// Trust the S/MIME signer certificate of the current message.
    TrustSigner,
    // ── Folder management ─────────────────────────────────────────────
    FolderNew,
    FolderRename,
    FolderDelete,
    FolderPromptInput(char),
    FolderPromptBackspace,
    FolderPromptSubmit,
    FolderPromptCancel,
    // ── Multi-select ──────────────────────────────────────────────────
    ToggleSelect,
    /// Reverse the last destructive action.
    Undo,
    /// Move the selection to the archive folder.
    Archive,
    /// Hide the replies of the cursor's thread.
    CollapseThread,
    /// Restore a collapsed thread.
    ExpandThread,
    /// Permanently remove every message in the current folder.
    EmptyFolder,
    // ── In-message search ─────────────────────────────────────────────
    SearchMessage,
    MessageSearchInput(char),
    MessageSearchBackspace,
    MessageSearchSubmit,
    MessageSearchCancel,
    NextMatch,
    PrevMatch,
    ClearSelection,
    // ── Sieve filters ─────────────────────────────────────────────────
    OpenSieve,
    SieveNext,
    SievePrev,
    SieveActivate,
    SieveDeactivate,
    SieveEdit,
    SieveNew,
    SieveDelete,
    SieveSave,
    SieveClose,
    SieveEscape,
    SieveNameInput(char),
    SieveNameBackspace,
    SieveNameSubmit,
    SieveNameCancel,
    SieveEditorKey(crossterm::event::KeyEvent),
    // ── Attachment list ───────────────────────────────────────────────
    OpenAttachments,
    AttachmentNext,
    AttachmentPrev,
    AttachmentOpen,
    AttachmentSave,
    AttachmentClose,
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
