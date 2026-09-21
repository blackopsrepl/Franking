/*! Remote IMAP/SMTP backend module wiring. */

mod errors;
mod folders;
mod mailboxes;
mod model;
pub mod next;
mod ops;
mod send;
mod smtp;
mod sync;
mod template;

#[cfg(test)]
mod tests;

pub use model::ImapSmtpService;
