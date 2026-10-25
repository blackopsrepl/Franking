/*! Compose module wiring. */

mod focus;
mod schedule;
mod state;
mod template;

#[cfg(test)]
mod tests;

pub use focus::{FocusedField, ACTION_BAR};
pub use schedule::{describe_send_after, parse_delay, send_time_in};
pub use state::{AutocompleteState, ComposeMode, ComposeState};
pub use template::{body_is_empty, populate_from_template, reassemble_template, signed_body};
