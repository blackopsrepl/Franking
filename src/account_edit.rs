/*! Account add/edit form state. */

use crate::mail::account_store::AccountRecord;

/// State for the account add/edit form.
pub struct AccountEditState {
    /// Account being edited; `None` when adding.
    pub editing: bool,
    pub name: String,
    pub username: String,
    pub imap_host: String,
    pub imap_port: String,
    pub imap_security: ConnectionSecurity,
    pub smtp_host: String,
    pub smtp_port: String,
    pub smtp_security: ConnectionSecurity,
    /// Optional ManageSieve host and port (blank uses the IMAP host).
    pub sieve_host: String,
    pub sieve_port: String,
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
            imap_security: ConnectionSecurity::for_port(993),
            smtp_host: String::new(),
            smtp_port: "465".to_string(),
            smtp_security: ConnectionSecurity::for_port(465),
            sieve_host: String::new(),
            sieve_port: String::new(),
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
            imap_security: ConnectionSecurity::parse(record.imap_security.as_deref()),
            smtp_host: record.smtp_host.clone().unwrap_or_default(),
            smtp_port: record
                .smtp_port
                .map(|port| port.to_string())
                .unwrap_or_default(),
            smtp_security: ConnectionSecurity::parse(record.smtp_security.as_deref()),
            sieve_host: record.sieve_host.clone().unwrap_or_default(),
            sieve_port: record
                .sieve_port
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
            AccountField::SieveHost => Some(&mut self.sieve_host),
            AccountField::SievePort => Some(&mut self.sieve_port),
            AccountField::ClientId => Some(&mut self.client_id),
            AccountField::ClientSecret => Some(&mut self.client_secret),
            AccountField::Password => Some(&mut self.password),
            AccountField::Auth => None,
            AccountField::ImapSecurity | AccountField::SmtpSecurity => None,
            AccountField::Default | AccountField::Save | AccountField::Cancel => None,
        }
    }

    pub fn toggle_default(&mut self) {
        self.is_default = !self.is_default;
    }

    /// Cycle the security selector the focused field names, if any.
    pub fn cycle_security(&mut self) {
        match self.focused {
            AccountField::ImapSecurity => self.imap_security = self.imap_security.next(),
            AccountField::SmtpSecurity => self.smtp_security = self.smtp_security.next(),
            _ => {}
        }
    }

    /// Follow the port with its usual security, so the common cases need no
    /// attention; the selector still overrides this.
    pub fn sync_security_to_ports(&mut self) {
        if let Ok(port) = self.imap_port.trim().parse::<u16>() {
            self.imap_security = ConnectionSecurity::for_port(port);
        }
        if let Ok(port) = self.smtp_port.trim().parse::<u16>() {
            self.smtp_security = ConnectionSecurity::for_port(port);
        }
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

/// Parse an optional port, rejecting non-numeric values.
pub fn parse_optional_port(value: &str) -> Result<Option<u16>, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    trimmed
        .parse::<u16>()
        .map(Some)
        .map_err(|_| format!("Port '{trimmed}' is not a number."))
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

mod auth;
mod field;
mod security;

pub use auth::AuthMode;
pub use field::AccountField;
pub use security::ConnectionSecurity;

#[cfg(test)]
mod tests;
