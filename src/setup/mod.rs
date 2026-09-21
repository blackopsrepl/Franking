/*! Setup wizard module wiring. */

mod accounts;
mod config;
mod oauth_setup;
mod wizard;

#[cfg(test)]
mod tests;

pub use wizard::{print_account_status, run_wizard};
