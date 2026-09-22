/*! RFC 5322 / RFC 2369 address representation with decoded display names. */

/// A single mailbox with an optional decoded display name.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Address {
    pub name: Option<String>,
    pub email: Option<String>,
}

impl Address {
    pub fn new(name: Option<String>, email: Option<String>) -> Self {
        Self { name, email }
    }

    pub fn email(&self) -> Option<&str> {
        self.email.as_deref()
    }

    /// The `addr-spec` lowercased, suitable for contact de-duplication.
    pub fn normalized_email(&self) -> Option<String> {
        self.email
            .as_deref()
            .map(|email| email.trim().to_ascii_lowercase())
            .filter(|email| !email.is_empty())
    }

    pub fn display(&self) -> String {
        match (self.name.as_deref(), self.email.as_deref()) {
            (Some(name), Some(email)) if !name.is_empty() => format!("{name} <{email}>"),
            (_, Some(email)) => email.to_string(),
            (Some(name), None) => name.to_string(),
            (None, None) => String::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.name.as_deref().unwrap_or_default().is_empty()
            && self.email.as_deref().unwrap_or_default().is_empty()
    }
}
