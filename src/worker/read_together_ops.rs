/*! Loading several messages for one combined scroll. */

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
}
