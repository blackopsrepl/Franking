/*! Application module wiring. */

mod accessors;
mod account_form;
mod accounts;
mod archive;
mod attachments;
mod autosave;
mod compose;
mod compose_actions;
mod compose_attachments;
mod contacts;
mod crypto;
mod editor;
mod empty_folder;
mod file_picker;
mod folders;
mod identities;
mod input;
mod links;
mod loading;
mod message_search;
mod model;
mod mouse;
mod move_prompt;
mod navigation;
mod outbox;
mod pgp;
mod selection;
mod settings;
mod sieve;
mod sieve_actions;
mod smime;
mod threads;
mod undo;
mod worker;

#[cfg(test)]
mod tests;

pub use model::{App, PendingOpenCommand};
