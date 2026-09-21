/*! Compose module wiring. */

mod state;
mod template;

#[cfg(test)]
mod tests;

pub use state::{AutocompleteState, ComposeMode, ComposeState, FocusedField};
pub use template::{body_is_empty, populate_from_template, reassemble_template};
