#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    pub name: String,
    pub backend: String,
    pub default: bool,
}

pub fn is_local_maildir_account(account: &Account) -> bool {
    account.backend.eq_ignore_ascii_case("maildir")
}

pub fn preferred_account(accounts: &[Account]) -> Option<&Account> {
    accounts
        .iter()
        .find(|account| account.default)
        .or_else(|| {
            accounts
                .iter()
                .find(|account| !is_local_maildir_account(account))
        })
        .or_else(|| accounts.first())
}

pub fn sort_accounts(accounts: &mut [Account]) {
    accounts.sort_by(|left, right| {
        right
            .default
            .cmp(&left.default)
            .then_with(|| is_local_maildir_account(left).cmp(&is_local_maildir_account(right)))
            .then_with(|| left.name.cmp(&right.name))
    });
}

/// Semantic role of a mailbox, from RFC 6154 SPECIAL-USE where available and
/// otherwise inferred from common names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FolderRole {
    Inbox,
    Sent,
    Drafts,
    Trash,
    Archive,
    Junk,
    Flagged,
    All,
    #[default]
    Other,
}

impl FolderRole {
    /// Infer a role from a mailbox name when the server advertises no attribute.
    pub fn from_name(name: &str) -> Self {
        match name.to_ascii_lowercase().as_str() {
            "inbox" => Self::Inbox,
            "sent" | "sent items" | "sent messages" | "sent mail" | "inbox.sent" => Self::Sent,
            "drafts" | "draft" | "inbox.drafts" => Self::Drafts,
            "trash" | "deleted" | "deleted items" | "deleted messages" | "bin" | "inbox.trash" => {
                Self::Trash
            }
            "archive" | "archives" | "inbox.archive" => Self::Archive,
            "spam" | "junk" | "bulk mail" => Self::Junk,
            "starred" | "flagged" => Self::Flagged,
            "all mail" | "all" => Self::All,
            _ => Self::Other,
        }
    }

    pub fn description(self) -> Option<&'static str> {
        match self {
            Self::Inbox => Some("Incoming messages"),
            Self::Sent => Some("Sent messages"),
            Self::Drafts => Some("Draft messages"),
            Self::Trash => Some("Deleted messages"),
            Self::Archive => Some("Archived messages"),
            Self::Junk => Some("Spam and junk"),
            Self::Flagged => Some("Flagged messages"),
            Self::All => Some("All messages"),
            Self::Other => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Folder {
    pub name: String,
    pub desc: Option<String>,
    pub role: FolderRole,
    /// Whether the account is subscribed to this mailbox, when the backend
    /// reports subscriptions.
    pub subscribed: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Envelope {
    pub id: String,
    pub flags: Vec<String>,
    pub subject: String,
    pub sender: Sender,
    pub date: String,
    /// RFC 5322 Message-ID, bracket-stripped.
    pub message_id: Option<String>,
    /// Parent Message-ID from In-Reply-To, bracket-stripped.
    pub in_reply_to: Option<String>,
    /// Source account, set when listed (used by the unified inbox).
    pub account: Option<String>,
    /// Source folder, set when listed.
    pub folder: Option<String>,
}

/// The changes a server reported for a cached folder since a sync anchor.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FolderDelta {
    /// True when the anchor could not be used and a full listing is required.
    pub full_resync: bool,
    pub uid_validity: Option<u32>,
    pub uid_next: Option<u32>,
    pub highest_modseq: Option<u64>,
    /// UIDs the server no longer has.
    pub vanished: Vec<u32>,
    /// UIDs whose flags changed, with their current flags.
    pub changed_flags: Vec<(u32, Vec<String>)>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Sender {
    Plain(String),
    Structured {
        name: Option<String>,
        addr: Option<String>,
    },
    #[default]
    Unknown,
}

impl Sender {
    pub fn display(&self) -> String {
        match self {
            Sender::Plain(s) => s.clone(),
            Sender::Structured { name, addr } => {
                if let Some(name) = name {
                    if !name.is_empty() {
                        return name.clone();
                    }
                }
                addr.clone().unwrap_or_default()
            }
            Sender::Unknown => String::new(),
        }
    }
}

impl std::fmt::Display for Sender {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display())
    }
}

impl Envelope {
    pub fn is_seen(&self) -> bool {
        self.flags
            .iter()
            .any(|flag| flag.eq_ignore_ascii_case("seen"))
    }

    pub fn is_flagged(&self) -> bool {
        self.flags
            .iter()
            .any(|flag| flag.eq_ignore_ascii_case("flagged"))
    }

    pub fn is_answered(&self) -> bool {
        self.flags
            .iter()
            .any(|flag| flag.eq_ignore_ascii_case("answered"))
    }

    pub fn flag_icon(&self) -> &'static str {
        if self.is_flagged() {
            "!"
        } else if !self.is_seen() {
            "\u{25cf}"
        } else if self.is_answered() {
            "\u{21a9}"
        } else {
            " "
        }
    }

    pub fn sender_display(&self) -> String {
        self.sender.display()
    }
}

#[cfg(test)]
mod tests {
    use super::{preferred_account, sort_accounts, Account, FolderRole};

    #[test]
    fn preferred_account_uses_real_account_before_local_test_fallback() {
        let accounts = vec![
            Account {
                name: "test".to_string(),
                backend: "maildir".to_string(),
                default: false,
            },
            Account {
                name: "work".to_string(),
                backend: "imap".to_string(),
                default: false,
            },
        ];

        assert_eq!(
            preferred_account(&accounts).map(|account| account.name.as_str()),
            Some("work")
        );
    }

    #[test]
    fn sort_accounts_keeps_real_accounts_ahead_of_local_maildir_fallbacks() {
        let mut accounts = vec![
            Account {
                name: "test".to_string(),
                backend: "maildir".to_string(),
                default: false,
            },
            Account {
                name: "zeta".to_string(),
                backend: "imap".to_string(),
                default: false,
            },
            Account {
                name: "alpha".to_string(),
                backend: "imap".to_string(),
                default: false,
            },
        ];

        sort_accounts(&mut accounts);

        assert_eq!(
            accounts
                .iter()
                .map(|account| account.name.as_str())
                .collect::<Vec<_>>(),
            vec!["alpha", "zeta", "test"]
        );
    }

    #[test]
    fn folder_role_infers_from_common_names() {
        assert_eq!(FolderRole::from_name("INBOX"), FolderRole::Inbox);
        assert_eq!(FolderRole::from_name("Sent Items"), FolderRole::Sent);
        assert_eq!(FolderRole::from_name("Drafts"), FolderRole::Drafts);
        assert_eq!(FolderRole::from_name("Deleted Items"), FolderRole::Trash);
        assert_eq!(FolderRole::from_name("Junk"), FolderRole::Junk);
        assert_eq!(FolderRole::from_name("Projects"), FolderRole::Other);
    }
}
