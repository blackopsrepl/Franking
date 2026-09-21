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
mod folder_jump;
mod folders;
mod identities;
mod input;
mod invite;
mod links;
mod loading;
mod message_loaded;
mod message_search;
mod model;
mod mouse;
mod move_prompt;
mod navigation;
mod outbox;
mod pgp;
pub(crate) mod pgp_keys;
mod selection;
mod settings;
mod sieve;
mod sieve_actions;
mod smime;
mod sort_order;
mod threads;
mod undo;
mod unread;
mod worker;

#[cfg(test)]
mod tests;

pub use model::{App, PendingOpenCommand};
