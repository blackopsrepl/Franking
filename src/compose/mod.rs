/*! Compose module wiring. */

mod schedule;
mod state;
mod template;

#[cfg(test)]
mod tests;

pub use schedule::{describe_send_after, parse_delay, send_time_in};
pub use state::{AutocompleteState, ComposeMode, ComposeState, FocusedField};
pub use template::{body_is_empty, populate_from_template, reassemble_template, signed_body};
