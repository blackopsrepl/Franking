/*! Mailbox lifecycle actions dispatched to the service. */

use super::dispatch::Worker;

impl Worker {
    pub fn create_folder(&self, account: Option<String>, name: String) {
        self.spawn_action(move |service| {
            service
                .create_folder(account.as_deref(), &name)
                .map(|()| format!("Created folder {name}."))
        });
    }

    pub fn rename_folder(&self, account: Option<String>, from: String, to: String) {
        self.spawn_action(move |service| {
            service
                .rename_folder(account.as_deref(), &from, &to)
                .map(|()| format!("Renamed {from} to {to}."))
        });
    }

    pub fn empty_folder(&self, account: Option<String>, name: String) {
        self.spawn_action(move |service| service.empty_folder(account.as_deref(), &name));
    }

    pub fn delete_folder(&self, account: Option<String>, name: String) {
        self.spawn_action(move |service| {
            service
                .delete_folder(account.as_deref(), &name)
                .map(|()| format!("Deleted folder {name}."))
        });
    }
}
