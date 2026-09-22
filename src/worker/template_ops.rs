/*! Compose template fetches dispatched to the service. */

use std::thread;

use super::dispatch::{Worker, WorkerResult};

impl Worker {
    /// Fetch a compose template (new message).
    pub fn copy_message(
        &self,
        account: Option<String>,
        folder: String,
        target: String,
        id: String,
    ) {
        self.spawn_action(move |service| {
            service
                .copy_message(account.as_deref(), &folder, &target, &id)
                .map(|()| format!("Copied to {target}."))
        });
    }

    /// Download every attachment of a message as one archive.
    pub fn download_attachments_zip(&self, account: Option<String>, folder: String, id: String) {
        self.spawn(
            move |service| service.download_attachments_zip(account.as_deref(), &folder, &id),
            WorkerResult::ActionDone,
        );
    }

    pub fn download_attachments(&self, account: Option<String>, folder: String, id: String) {
        self.spawn_action(move |service| {
            service
                .download_attachments(account.as_deref(), &folder, &id)
                .map(|saved| format!("Attachments: {}", saved.trim()))
        });
    }

    pub fn fetch_template_write(&self, account: Option<String>) {
        let tx = self.tx.clone();
        let service = self.service.clone();
        thread::spawn(move || {
            let result = service.template_write(account.as_deref());
            let _ = tx.send(WorkerResult::Template(result));
        });
    }

    /// Fetch a reply template.
    pub fn fetch_template_reply(
        &self,
        account: Option<String>,
        folder: String,
        id: String,
        all: bool,
    ) {
        let tx = self.tx.clone();
        let service = self.service.clone();
        thread::spawn(move || {
            let result = service.template_reply(account.as_deref(), &folder, &id, all);
            let _ = tx.send(WorkerResult::Template(result));
        });
    }

    /// Fetch a resume template for a stored draft.
    pub fn fetch_draft_template(&self, account: Option<String>, folder: String, id: String) {
        let tx = self.tx.clone();
        let service = self.service.clone();
        thread::spawn(move || {
            let result = service.draft_template(account.as_deref(), &folder, &id);
            let _ = tx.send(WorkerResult::Template(result));
        });
    }

    /// Fetch a forward template.
    pub fn fetch_template_forward(&self, account: Option<String>, folder: String, id: String) {
        let tx = self.tx.clone();
        let service = self.service.clone();
        thread::spawn(move || {
            let result = service.template_forward(account.as_deref(), &folder, &id);
            let _ = tx.send(WorkerResult::Template(result));
        });
    }
}
