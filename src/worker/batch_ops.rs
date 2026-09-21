/*! Batch operations over selected messages.
The service stays per-message; batching loops in one background job so the
UI reports a single summary and the mailbox is touched once per message. */

use super::dispatch::Worker;

/// Human summary for a batch result.
fn summary(verb: &str, total: usize, failed: usize) -> String {
    let noun = if total == 1 { "message" } else { "messages" };
    if failed == 0 {
        format!("{verb} {total} {noun}.")
    } else {
        format!(
            "{verb} {} of {total} {noun}; {failed} failed.",
            total - failed
        )
    }
}

impl Worker {
    pub fn delete_messages(&self, account: Option<String>, folder: String, ids: Vec<String>) {
        let total = ids.len();
        self.spawn_action(move |service| {
            let mut failed = 0;
            for id in &ids {
                if service
                    .delete_message(account.as_deref(), &folder, id)
                    .is_err()
                {
                    failed += 1;
                }
            }
            Ok(summary("Deleted", total, failed))
        });
    }

    pub fn move_messages(
        &self,
        account: Option<String>,
        folder: String,
        target: String,
        ids: Vec<String>,
    ) {
        let total = ids.len();
        self.spawn_action(move |service| {
            let mut failed = 0;
            for id in &ids {
                if service
                    .move_message(account.as_deref(), &folder, &target, id)
                    .is_err()
                {
                    failed += 1;
                }
            }
            Ok(summary(&format!("Moved to {target}:"), total, failed))
        });
    }

    pub fn copy_messages(
        &self,
        account: Option<String>,
        folder: String,
        target: String,
        ids: Vec<String>,
    ) {
        let total = ids.len();
        self.spawn_action(move |service| {
            let mut failed = 0;
            for id in &ids {
                if service
                    .copy_message(account.as_deref(), &folder, &target, id)
                    .is_err()
                {
                    failed += 1;
                }
            }
            Ok(summary(&format!("Copied to {target}:"), total, failed))
        });
    }

    pub fn flag_messages(
        &self,
        account: Option<String>,
        folder: String,
        ids: Vec<String>,
        flag: String,
        add: bool,
    ) {
        let total = ids.len();
        self.spawn_action(move |service| {
            let mut failed = 0;
            for id in &ids {
                let result = if add {
                    service.flag_add(account.as_deref(), &folder, id, &flag)
                } else {
                    service.flag_remove(account.as_deref(), &folder, id, &flag)
                };
                if result.is_err() {
                    failed += 1;
                }
            }
            let verb = if add { "Flagged" } else { "Unflagged" };
            Ok(summary(verb, total, failed))
        });
    }
}

#[cfg(test)]
mod tests {
    use super::summary;

    #[test]
    fn summarizes_success_and_partial_failure() {
        assert_eq!(summary("Deleted", 1, 0), "Deleted 1 message.");
        assert_eq!(summary("Deleted", 3, 0), "Deleted 3 messages.");
        assert_eq!(
            summary("Deleted", 3, 1),
            "Deleted 2 of 3 messages; 1 failed."
        );
    }
}
