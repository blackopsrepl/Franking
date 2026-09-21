/*! Account add/edit form state. */

use crate::mail::account_store::AccountRecord;

/// Authentication method for a new or edited account.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AuthMode {
    #[default]
    Password,
    /// Google OAuth2 (Gmail / Google Workspace).
    GmailOAuth,
    /// Microsoft OAuth2 (Outlook / Microsoft 365).
    OutlookOAuth,
}

impl AuthMode {
    pub fn label(self) -> &'static str {
        match self {
            AuthMode::Password => "password / app password",
            AuthMode::GmailOAuth => "Google OAuth2",
            AuthMode::OutlookOAuth => "Microsoft OAuth2",
        }
    }

    pub fn provider_kind(self) -> Option<&'static str> {
        match self {
            AuthMode::Password => None,
            AuthMode::GmailOAuth => Some("gmail"),
            AuthMode::OutlookOAuth => Some("outlook"),
        }
    }

    pub fn cycle(self) -> Self {
        match self {
            AuthMode::Password => AuthMode::GmailOAuth,
            AuthMode::GmailOAuth => AuthMode::OutlookOAuth,
            AuthMode::OutlookOAuth => AuthMode::Password,
        }
    }
}

/// Focused field in the account form.
///
/// Tab cycle: Name → Username → ImapHost → ImapPort → SmtpHost → SmtpPort
///            → Password → Default → Save → Cancel → Name
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountField {
    Name,
    Username,
    ImapHost,
    ImapPort,
    SmtpHost,
    SmtpPort,
    Auth,
    ClientId,
    ClientSecret,
    Password,
    Default,
    Save,
    Cancel,
}

impl AccountField {
    fn next(self) -> Self {
        match self {
            AccountField::Name => AccountField::Username,
            AccountField::Username => AccountField::ImapHost,
            AccountField::ImapHost => AccountField::ImapPort,
            AccountField::ImapPort => AccountField::SmtpHost,
            AccountField::SmtpHost => AccountField::SmtpPort,
            AccountField::SmtpPort => AccountField::Auth,
            AccountField::Auth => AccountField::ClientId,
            AccountField::ClientId => AccountField::ClientSecret,
            AccountField::ClientSecret => AccountField::Password,
            AccountField::Password => AccountField::Default,
            AccountField::Default => AccountField::Save,
            AccountField::Save => AccountField::Cancel,
            AccountField::Cancel => AccountField::Name,
        }
    }

    /// Advance (or retreat) the focus by `delta`.
    pub fn step(self, delta: i32) -> Self {
        if delta >= 0 {
            self.next()
        } else {
            match self {
                AccountField::Name => AccountField::Cancel,
                AccountField::Username => AccountField::Name,
                AccountField::ImapHost => AccountField::Username,
                AccountField::ImapPort => AccountField::ImapHost,
                AccountField::SmtpHost => AccountField::ImapPort,
                AccountField::SmtpPort => AccountField::SmtpHost,
                AccountField::Auth => AccountField::SmtpPort,
                AccountField::ClientId => AccountField::Auth,
                AccountField::ClientSecret => AccountField::ClientId,
                AccountField::Password => AccountField::ClientSecret,
                AccountField::Default => AccountField::Password,
                AccountField::Save => AccountField::Default,
                AccountField::Cancel => AccountField::Save,
            }
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            AccountField::Name => "Name    ",
            AccountField::Username => "Login   ",
            AccountField::ImapHost => "IMAP    ",
            AccountField::ImapPort => "IMAP pt ",
            AccountField::SmtpHost => "SMTP    ",
            AccountField::SmtpPort => "SMTP pt ",
            AccountField::Auth => "Auth    ",
            AccountField::ClientId => "Client  ",
            AccountField::ClientSecret => "Secret  ",
            AccountField::Password => "Password",
            AccountField::Default => "Default ",
            AccountField::Save => "Save",
            AccountField::Cancel => "Cancel",
        }
    }

    /// True for the action-bar buttons.
    pub fn is_action_button(self) -> bool {
        matches!(self, AccountField::Save | AccountField::Cancel)
    }
}

/// State for the account add/edit form.
pub struct AccountEditState {
    /// Account being edited; `None` when adding.
    pub editing: bool,
    pub name: String,
    pub username: String,
    pub imap_host: String,
    pub imap_port: String,
    pub smtp_host: String,
    pub smtp_port: String,
    /// Authentication method.
    pub auth_mode: AuthMode,
    pub client_id: String,
    pub client_secret: String,
    pub password: String,
    pub is_default: bool,
    pub focused: AccountField,
    pub error: Option<String>,
}

impl Default for AccountEditState {
    fn default() -> Self {
        Self::new()
    }
}

impl AccountEditState {
    pub fn new() -> Self {
        Self {
            editing: false,
            name: String::new(),
            username: String::new(),
            imap_host: String::new(),
            imap_port: "993".to_string(),
            smtp_host: String::new(),
            smtp_port: "465".to_string(),
            auth_mode: AuthMode::default(),
            client_id: String::new(),
            client_secret: String::new(),
            password: String::new(),
            is_default: false,
            focused: AccountField::Name,
            error: None,
        }
    }

    pub fn from_record(record: &AccountRecord) -> Self {
        Self {
            editing: true,
            name: record.name.clone(),
            username: record.username.clone().unwrap_or_default(),
            imap_host: record.imap_host.clone().unwrap_or_default(),
            imap_port: record
                .imap_port
                .map(|port| port.to_string())
                .unwrap_or_default(),
            smtp_host: record.smtp_host.clone().unwrap_or_default(),
            smtp_port: record
                .smtp_port
                .map(|port| port.to_string())
                .unwrap_or_default(),
            auth_mode: match record.auth_mode.as_deref() {
                Some("oauth2") => match record.provider_kind.as_str() {
                    "outlook" => AuthMode::OutlookOAuth,
                    _ => AuthMode::GmailOAuth,
                },
                _ => AuthMode::Password,
            },
            client_id: String::new(),
            client_secret: String::new(),
            password: String::new(),
            is_default: record.is_default,
            focused: AccountField::Name,
            error: None,
        }
    }

    /// Mutable reference to the focused text field.
    pub fn focused_field_mut(&mut self) -> Option<&mut String> {
        match self.focused {
            AccountField::Name => Some(&mut self.name),
            AccountField::Username => Some(&mut self.username),
            AccountField::ImapHost => Some(&mut self.imap_host),
            AccountField::ImapPort => Some(&mut self.imap_port),
            AccountField::SmtpHost => Some(&mut self.smtp_host),
            AccountField::SmtpPort => Some(&mut self.smtp_port),
            AccountField::ClientId => Some(&mut self.client_id),
            AccountField::ClientSecret => Some(&mut self.client_secret),
            AccountField::Password => Some(&mut self.password),
            AccountField::Auth => None,
            AccountField::Default | AccountField::Save | AccountField::Cancel => None,
        }
    }

    pub fn toggle_default(&mut self) {
        self.is_default = !self.is_default;
    }

    /// Cycle the authentication method.
    pub fn cycle_auth_mode(&mut self) {
        self.auth_mode = self.auth_mode.cycle();
    }

    /// Validated field values: name, username, imap host/port, smtp host/port.
    #[allow(clippy::type_complexity)]
    pub fn validate(&self) -> Result<(String, String, String, u16, String, u16), String> {
        let required = |value: &str, label: &str| -> Result<String, String> {
            let trimmed = value.trim().to_string();
            if trimmed.is_empty() {
                Err(format!("{label} is required."))
            } else {
                Ok(trimmed)
            }
        };
        let name = required(&self.name, "Account name")?;
        if name.contains(char::is_whitespace) {
            return Err("Account name must not contain spaces.".to_string());
        }
        let username = required(&self.username, "Login")?;
        let imap_host = required(&self.imap_host, "IMAP host")?;
        let smtp_host = required(&self.smtp_host, "SMTP host")?;
        let imap_port = parse_port(&self.imap_port, 993)?;
        let smtp_port = parse_port(&self.smtp_port, 465)?;
        Ok((name, username, imap_host, imap_port, smtp_host, smtp_port))
    }
}

fn parse_port(value: &str, default: u16) -> Result<u16, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(default);
    }
    trimmed
        .parse()
        .map_err(|_| format!("Port '{trimmed}' is not a number."))
}
