/*! Loading several messages for one combined scroll, and one message for the
sequential reply queue. */

use std::thread;

use super::dispatch::{Worker, WorkerResult};

impl Worker {
    pub fn fetch_read_together(&self, targets: Vec<(String, String, String)>) {
        self.spawn(
            move |service| {
                let mut documents = Vec::new();
                for (account, folder, id) in &targets {
                    match service.read_message_content(Some(account), folder, id) {
                        Ok(document) => documents.push(document),
                        Err(error) => return Err(error),
                    }
                }
                Ok(documents)
            },
            WorkerResult::ReadTogether,
        );
    }

    /// Load the message the sequential reply queue is showing.
    pub fn fetch_focus_message(&self, account: Option<String>, folder: String, id: String) {
        let tx = self.tx.clone();
        let service = self.service.clone();
        thread::spawn(move || {
            let result = service.read_message_content(account.as_deref(), &folder, &id);
            let _ = tx.send(WorkerResult::FocusMessage(Box::new(result)));
        });
    }
}
