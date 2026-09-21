/* Identity add/edit form state. */

use crate::identities::Identity;

/// Which field in the identity edit form currently has focus.
///
/// Tab cycle: Name → SenderName → Email → IsDefault → Save → Cancel → Name
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityField {
    /// Short UI label to identify this identity (e.g. "Work", "Personal").
    Name,
    /// Sender display name that goes into the From header (e.g. "Alice Example").
    SenderName,
    Email,
    Signature,
    IsDefault,
    // ── Action bar buttons ─────────────────────────────────────────
    Save,
    Cancel,
}

impl IdentityField {
    pub fn next(self) -> Self {
        match self {
            IdentityField::Name => IdentityField::SenderName,
            IdentityField::SenderName => IdentityField::Email,
            IdentityField::Email => IdentityField::Signature,
            IdentityField::Signature => IdentityField::IsDefault,
            IdentityField::IsDefault => IdentityField::Save,
            IdentityField::Save => IdentityField::Cancel,
            IdentityField::Cancel => IdentityField::Name,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            IdentityField::Name => IdentityField::Cancel,
            IdentityField::SenderName => IdentityField::Name,
            IdentityField::Email => IdentityField::SenderName,
            IdentityField::Signature => IdentityField::Email,
            IdentityField::IsDefault => IdentityField::Signature,
            IdentityField::Save => IdentityField::IsDefault,
            IdentityField::Cancel => IdentityField::Save,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            IdentityField::Name => "Name   ",
            IdentityField::SenderName => "Sender ",
            IdentityField::Email => "Email  ",
            IdentityField::Signature => "Signat.",
            IdentityField::IsDefault => "Default",
            IdentityField::Save => "Save",
            IdentityField::Cancel => "Cancel",
        }
    }

    /// True if this is an action-bar button.
    pub fn is_action_button(self) -> bool {
        matches!(self, IdentityField::Save | IdentityField::Cancel)
    }
}

/// State for the identity add/edit form.
pub struct IdentityEditState {
    /// ID of the identity being edited. `None` for new identities.
    pub identity_id: Option<i64>,
    /// Account this identity belongs to (read-only; not editable in-form).
    pub account: String,
    /// Short UI label (e.g. "Work"). Required.
    pub name: String,
    /// Sender display name for the From header (e.g. "Alice Example"). Optional.
    pub display_name: String,
    pub email: String,
    /// Signature appended to new messages (optional).
    pub signature: String,
    pub is_default: bool,
    /// Currently focused field.
    pub focused: IdentityField,
    /// Validation error message.
    pub error: Option<String>,
}

impl IdentityEditState {
    /// Create a blank form for a new identity on `account`.
    pub fn new(account: &str) -> Self {
        Self {
            identity_id: None,
            account: account.to_string(),
            name: String::new(),
            display_name: String::new(),
            email: String::new(),
            signature: String::new(),
            is_default: false,
            focused: IdentityField::Name,
            error: None,
        }
    }

    /// Pre-fill the form from an existing identity.
    pub fn from_identity(identity: &Identity) -> Self {
        Self {
            identity_id: Some(identity.id),
            account: identity.account.clone(),
            name: identity.name.clone().unwrap_or_default(),
            display_name: identity.display_name.clone().unwrap_or_default(),
            email: identity.email.clone(),
            signature: identity.signature.clone().unwrap_or_default(),
            is_default: identity.is_default,
            focused: IdentityField::Name,
            error: None,
        }
    }

    /// Mutable reference to the currently focused text field.
    /// Returns `None` for toggle fields (IsDefault) and action-bar buttons.
    pub fn focused_field_mut(&mut self) -> Option<&mut String> {
        match self.focused {
            IdentityField::Name => Some(&mut self.name),
            IdentityField::SenderName => Some(&mut self.display_name),
            IdentityField::Email => Some(&mut self.email),
            IdentityField::Signature => Some(&mut self.signature),
            IdentityField::IsDefault => None,
            IdentityField::Save | IdentityField::Cancel => None,
        }
    }

    /// Toggle the is_default boolean (for the IsDefault field).
    pub fn toggle_default(&mut self) {
        self.is_default = !self.is_default;
    }

    /// Validate and return the field values for saving.
    /// Returns `Err` if validation fails.
    #[allow(clippy::type_complexity)]
    pub fn validate(
        &self,
    ) -> Result<(Option<String>, Option<String>, String, Option<String>, bool), String> {
        let name = {
            let n = self.name.trim().to_string();
            if n.is_empty() {
                None
            } else {
                Some(n)
            }
        };
        let email = self.email.trim().to_string();
        if email.is_empty() || !email.contains('@') {
            return Err("Email is required and must contain '@'.".to_string());
        }
        let display_name = {
            let n = self.display_name.trim().to_string();
            if n.is_empty() {
                None
            } else {
                Some(n)
            }
        };
        let signature = {
            let text = self.signature.trim().to_string();
            if text.is_empty() {
                None
            } else {
                Some(text)
            }
        };
        Ok((name, display_name, email, signature, self.is_default))
    }
}
