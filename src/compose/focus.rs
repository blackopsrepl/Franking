/*! Compose focus: header fields, the body, and the action-bar buttons. */

/// Which field / action-bar button currently has focus.
///
/// Tab cycles From → To → Cc → Bcc → Subject → Body, then every button in
/// [`ACTION_BAR`] order, and wraps back to From.
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
    SendLater,
    Draft,
    Attach,
    Files,
    Sign,
    Encrypt,
    SmimeSign,
    SmimeEncrypt,
    Discard,
}

const FIELDS: [FocusedField; 6] = [
    FocusedField::From,
    FocusedField::To,
    FocusedField::Cc,
    FocusedField::Bcc,
    FocusedField::Subject,
    FocusedField::Body,
];

/// Action-bar buttons in display and Tab order. Rendering and focus cycling
/// both read this one list, so a button cannot be focusable but invisible.
pub const ACTION_BAR: [FocusedField; 10] = [
    FocusedField::Send,
    FocusedField::SendLater,
    FocusedField::Draft,
    FocusedField::Attach,
    FocusedField::Files,
    FocusedField::Sign,
    FocusedField::Encrypt,
    FocusedField::SmimeSign,
    FocusedField::SmimeEncrypt,
    FocusedField::Discard,
];

impl FocusedField {
    fn cycle() -> impl Iterator<Item = FocusedField> + Clone {
        FIELDS.into_iter().chain(ACTION_BAR)
    }

    fn position(self) -> usize {
        Self::cycle()
            .position(|field| field == self)
            .expect("every field is in the cycle")
    }

    pub fn next(self) -> Self {
        let len = FIELDS.len() + ACTION_BAR.len();
        Self::cycle()
            .nth((self.position() + 1) % len)
            .expect("index is in range")
    }

    pub fn prev(self) -> Self {
        let len = FIELDS.len() + ACTION_BAR.len();
        Self::cycle()
            .nth((self.position() + len - 1) % len)
            .expect("index is in range")
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
            FocusedField::SendLater => "Send later",
            FocusedField::Draft => "Draft",
            FocusedField::Attach => "Attach",
            FocusedField::Files => "Files",
            FocusedField::Sign => "Sign",
            FocusedField::Encrypt => "Encrypt",
            FocusedField::SmimeSign => "S/MIME Sign",
            FocusedField::SmimeEncrypt => "S/MIME Encrypt",
            FocusedField::Discard => "Discard",
        }
    }

    /// True if this is an action-bar button (not a text/content field).
    pub fn is_action_button(self) -> bool {
        ACTION_BAR.contains(&self)
    }

    /// Index of this button in [`ACTION_BAR`], if it is one.
    pub fn action_index(self) -> Option<usize> {
        ACTION_BAR.iter().position(|button| *button == self)
    }
}
