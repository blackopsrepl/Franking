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
mod links;
mod loading;
mod local_search;
mod message_loaded;
mod message_search;
mod model;
mod mouse;
mod move_prompt;
mod navigation;
mod notification_rules;
mod outbox;
mod pgp;
pub(crate) mod pgp_keys;
mod planner;
mod saved_searches;
mod search_flow;
mod selection;
mod settings;
mod sieve;
mod sieve_actions;
mod smime;
mod sort_order;
mod threads;
mod triage;
mod undo;
mod unread;
mod worker;

#[cfg(test)]
mod tests;

pub use model::{App, PendingOpenCommand};
