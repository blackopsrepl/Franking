/*! App unit test wiring. */

mod accounts;
mod actions_menu;
mod archive;
mod attachment_library;
mod attachment_save;
mod compose;
mod compose_escape;
mod conversation;
mod copy_move;
mod crypto;
mod help;
mod keys;
mod mail_ui;
mod message_search;
mod move_prompt;
mod notifications;
mod reply_later;
mod saved_searches;
mod settings;
mod sieve;
mod sieve_settings;
mod snippets;
mod undo;
mod unread;

/// An Inbox envelope from `sender`, received by `account`.
pub(super) fn envelope(account: &str, id: &str, sender: &str) -> crate::mail::types::Envelope {
    crate::mail::types::Envelope {
        id: id.into(),
        flags: Vec::new(),
        subject: id.into(),
        sender: crate::mail::types::Sender::Structured {
            name: None,
            addr: Some(sender.into()),
        },
        date: "2026-09-23".into(),
        message_id: None,
        in_reply_to: None,
        account: Some(account.into()),
        folder: Some("INBOX".into()),
    }
}
