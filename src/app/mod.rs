/*! Application module wiring. */

mod attachments;
mod compose;
mod contacts;
mod crypto;
mod editor;
mod identities;
mod input;
mod loading;
mod model;
mod mouse;
mod navigation;
mod pgp;
mod smime;
mod worker;

#[cfg(test)]
mod tests;

pub use model::{App, PendingOpenCommand};
