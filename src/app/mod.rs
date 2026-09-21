/*! Application module wiring. */

mod accessors;
mod attachments;
mod compose;
mod contacts;
mod crypto;
mod editor;
mod folders;
mod identities;
mod input;
mod loading;
mod model;
mod mouse;
mod navigation;
mod pgp;
mod selection;
mod sieve;
mod smime;
mod worker;

#[cfg(test)]
mod tests;

pub use model::{App, PendingOpenCommand};
