/*! Mailbox lifecycle actions dispatched to the service. */

use super::dispatch::{Worker, WorkerResult};
use crate::mail::MailError;

/// A completed server folder rename or delete, so rules can follow it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FolderChange {
    Renamed {
        account: Option<String>,
        from: String,
        to: String,
        /// Hierarchy delimiter, so rules on subfolders follow the rename.
        delimiter: Option<char>,
    },
    Deleted {
        account: Option<String>,
        name: String,
    },
}

/// What resuming a rule needs from the server: whether its folder still
/// exists and the live INBOX `(UIDVALIDITY, UIDNEXT)`.
pub type ResumeCheck = Result<(bool, Option<u32>, Option<u32>), MailError>;

impl Worker {
    /// Check a suspended rule against its own account's server state.
    pub fn check_rule_resume(&self, rule: crate::db::folder_rules::FolderRule) {
        let (account, target) = (rule.account.clone(), rule.target.clone());
        self.spawn(
            move |service| {
                let exists = service
                    .list_folders(Some(&account))?
                    .iter()
                    .any(|folder| folder.name == target);
                let (validity, next) = service.folder_sync_cursor(Some(&account), "INBOX")?;
                Ok((exists, validity, next))
            },
            move |result| WorkerResult::RuleResumeChecked(rule, result),
        );
    }

    pub fn create_folder(&self, account: Option<String>, name: String) {
        self.spawn_action(move |service| {
            service
                .create_folder(account.as_deref(), &name)
                .map(|()| format!("Created folder {name}."))
        });
    }

    pub fn rename_folder(&self, account: Option<String>, from: String, to: String) {
        self.spawn(
            {
                let (account, from, to) = (account.clone(), from.clone(), to.clone());
                move |service| service.rename_folder(account.as_deref(), &from, &to)
            },
            move |result| {
                let delimiter = result.as_ref().ok().copied().flatten();
                let message = format!("Renamed {from} to {to}.");
                let change = FolderChange::Renamed {
                    account,
                    from,
                    to,
                    delimiter,
                };
                WorkerResult::FolderChanged(change, result.map(|_| message))
            },
        );
    }

    pub fn empty_folder(&self, account: Option<String>, name: String) {
        self.spawn_action(move |service| service.empty_folder(account.as_deref(), &name));
    }

    pub fn delete_folder(&self, account: Option<String>, name: String) {
        self.spawn(
            {
                let (account, name) = (account.clone(), name.clone());
                move |service| {
                    service
                        .delete_folder(account.as_deref(), &name)
                        .map(|()| format!("Deleted folder {name}."))
                }
            },
            move |result| {
                WorkerResult::FolderChanged(FolderChange::Deleted { account, name }, result)
            },
        );
    }
}
