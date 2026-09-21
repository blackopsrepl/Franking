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
                    sign: item.sign,
                    encrypt: item.encrypt,
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
