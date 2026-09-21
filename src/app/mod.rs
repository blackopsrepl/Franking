/*! Application module wiring. */

mod accessors;
mod archive;
mod attachments;
mod autosave;
mod compose;
mod contacts;
mod crypto;
mod editor;
mod empty_folder;
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
mod pgp;
mod selection;
mod sieve;
mod smime;
mod threads;
mod undo;
mod worker;

#[cfg(test)]
mod tests;

pub use model::{App, PendingOpenCommand};
