/*! ManageSieve actions dispatched to the service. */

use super::dispatch::{Worker, WorkerResult};

impl Worker {
    /// Fetch the account's Sieve scripts.
    pub fn fetch_sieve_scripts(&self, account: Option<String>) {
        self.spawn(
            move |service| service.sieve_scripts(account.as_deref()),
            WorkerResult::SieveScripts,
        );
    }

    /// Fetch one script's source for editing.
    pub fn fetch_sieve_script(&self, account: Option<String>, name: String) {
        let label = name.clone();
        self.spawn(
            move |service| service.sieve_script(account.as_deref(), &name),
            move |result| WorkerResult::SieveBody(label, result),
        );
    }

    pub fn sieve_save_script(&self, account: Option<String>, name: String, body: String) {
        self.spawn_action(move |service| {
            service
                .sieve_save_script(account.as_deref(), &name, &body)
                .map(|()| format!("Saved script {name}."))
        });
    }

    pub fn sieve_set_active(&self, account: Option<String>, name: Option<String>) {
        self.spawn_action(move |service| {
            service
                .sieve_set_active(account.as_deref(), name.as_deref())
                .map(|()| match &name {
                    Some(name) => format!("Activated script {name}."),
                    None => "Deactivated all scripts.".to_string(),
                })
        });
    }

    pub fn sieve_delete_script(&self, account: Option<String>, name: String) {
        self.spawn_action(move |service| {
            service
                .sieve_delete_script(account.as_deref(), &name)
                .map(|()| format!("Deleted script {name}."))
        });
    }
}
