/*! Outbox actions: list, flush, and discard queued messages. */

use crate::mail::errors::MailError;
use crate::mail::outbox;
use crate::mail::service::SendOptions;

use super::dispatch::{Worker, WorkerResult};

impl Worker {
    /// Load the queued messages.
    pub fn fetch_outbox(&self) {
        self.spawn(
            move |_service| {
                let conn = crate::db::open().map_err(db_error)?;
                outbox::list(&conn).map_err(outbox_error)
            },
            WorkerResult::Outbox,
        );
    }

    /// Send one queued message and remove it on success.
    pub fn send_outbox_item(&self, id: i64, passphrase: String) {
        self.spawn(
            move |service| {
                let conn = crate::db::open().map_err(db_error)?;
                let item = outbox::list(&conn)
                    .map_err(outbox_error)?
                    .into_iter()
                    .find(|item| item.id == id)
                    .ok_or_else(|| MailError::other(format!("outbox item {id} is gone")))?;

                let options = SendOptions {
                    sign: item.protection.sign,
                    encrypt: item.protection.encrypt,
                    smime_sign: item.protection.smime_sign,
                    smime_encrypt: item.protection.smime_encrypt,
                    passphrase,
                    keys_dir: Some(crate::mail::pgp::default_keys_dir()),
                };
                let status =
                    service.template_send(item.account.as_deref(), &item.template, &options)?;
                outbox::delete(&conn, id).map_err(outbox_error)?;
                Ok(status)
            },
            WorkerResult::SendDone,
        );
    }

    /// Send every queued message whose scheduled time has arrived.
    pub fn flush_outbox(&self, passphrase: String) {
        self.spawn(
            move |service| {
                let conn = crate::db::open().map_err(db_error)?;
                let now = chrono::Local::now().to_rfc3339();
                let due = outbox::due(&conn, &now).map_err(outbox_error)?;
                let total = due.len();
                if total == 0 {
                    return Ok("No queued messages are due.".to_string());
                }
                let mut sent = 0;
                for item in due {
                    let options = SendOptions {
                        sign: item.protection.sign,
                        encrypt: item.protection.encrypt,
                        smime_sign: item.protection.smime_sign,
                        smime_encrypt: item.protection.smime_encrypt,
                        passphrase: passphrase.clone(),
                        keys_dir: Some(crate::mail::pgp::default_keys_dir()),
                    };
                    if service
                        .template_send(item.account.as_deref(), &item.template, &options)
                        .is_ok()
                    {
                        outbox::delete(&conn, item.id).map_err(outbox_error)?;
                        sent += 1;
                    }
                }
                Ok(format!("Sent {sent} of {total} queued message(s)."))
            },
            WorkerResult::ActionDone,
        );
    }

    /// Drop one queued message without sending it.
    pub fn discard_outbox_item(&self, id: i64) {
        self.spawn_action(move |_service| {
            let conn = crate::db::open().map_err(db_error)?;
            outbox::delete(&conn, id).map_err(outbox_error)?;
            Ok("Queued message discarded.".to_string())
        });
    }
}

fn db_error(error: anyhow::Error) -> MailError {
    MailError::other(error.to_string())
}

fn outbox_error(error: anyhow::Error) -> MailError {
    MailError::other(error.to_string())
}
