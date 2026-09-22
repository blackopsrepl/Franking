/*! Focused field in the account form, and the tab order through it. */

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
    ImapSecurity,
    SmtpHost,
    SmtpPort,
    SmtpSecurity,
    SieveHost,
    SievePort,
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
            AccountField::ImapPort => AccountField::ImapSecurity,
            AccountField::ImapSecurity => AccountField::SmtpHost,
            AccountField::SmtpHost => AccountField::SmtpPort,
            AccountField::SmtpPort => AccountField::SmtpSecurity,
            AccountField::SmtpSecurity => AccountField::SieveHost,
            AccountField::SieveHost => AccountField::SievePort,
            AccountField::SievePort => AccountField::Auth,
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
                AccountField::ImapSecurity => AccountField::ImapPort,
                AccountField::SmtpHost => AccountField::ImapSecurity,
                AccountField::SmtpPort => AccountField::SmtpHost,
                AccountField::SmtpSecurity => AccountField::SmtpPort,
                AccountField::SieveHost => AccountField::SmtpSecurity,
                AccountField::SievePort => AccountField::SieveHost,
                AccountField::Auth => AccountField::SievePort,
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
            AccountField::ImapSecurity => "IMAP sec",
            AccountField::SmtpHost => "SMTP    ",
            AccountField::SmtpPort => "SMTP pt ",
            AccountField::SmtpSecurity => "SMTP sec",
            AccountField::SieveHost => "Sieve   ",
            AccountField::SievePort => "Sieve pt",
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
