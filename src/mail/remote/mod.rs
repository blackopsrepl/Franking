/*! Remote IMAP/SMTP backend module wiring. */

mod envelope;
mod errors;
mod folders;
mod mailboxes;
mod model;
mod ops;
mod roles;
mod search;
mod send;
mod smtp;
mod template;

#[cfg(test)]
mod tests;

pub use model::ImapSmtpService;
