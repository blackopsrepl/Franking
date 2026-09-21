/*! Application module wiring. */

mod compose;
mod contacts;
mod editor;
mod identities;
mod input;
mod loading;
mod model;
mod mouse;
mod navigation;
mod worker;

#[cfg(test)]
mod tests;

pub use model::{App, PendingOpenCommand};
