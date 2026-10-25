/*! Mailbox lifecycle actions dispatched to the service. */

use super::dispatch::{Worker, WorkerResult};

/// A completed server folder rename or delete, so rules can follow it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FolderChange {
    Renamed {
        account: Option<String>,
        from: String,
        to: String,
    },
    Deleted {
        account: Option<String>,
        name: String,
    },
}

impl Worker {
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
                move |service| {
                    service
                        .rename_folder(account.as_deref(), &from, &to)
                        .map(|()| format!("Renamed {from} to {to}."))
                }
            },
            move |result| {
                WorkerResult::FolderChanged(FolderChange::Renamed { account, from, to }, result)
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
