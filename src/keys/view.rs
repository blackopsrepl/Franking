/*! Views a key event can be dispatched against. */

/// The current view determines which keybindings are active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::enum_variant_names)]
pub enum View {
    EnvelopeList,
    MessageView,
    FolderList,
    AccountList,
    /// Account add/edit form.
    AccountEdit,
    /// Filesystem picker for attachment paths.
    FilePicker,
    /// Outbox of messages waiting to be sent.
    Outbox,
    /// Preferences overlay.
    Settings,
    /// Prompt for a send-later delay.
    SchedulePrompt,
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
    /// Link list overlay for the current message.
    LinkList,
    /// Text preview of one attachment.
    AttachmentView,
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
    /// PGP and S/MIME key material on disk.
    Keys,
    /// Key import path or key-generation identity prompt.
    KeysPrompt,
    /// Saved searches browser.
    SavedSearches,
    /// Naming the active search before saving it.
    SaveSearch,
}
