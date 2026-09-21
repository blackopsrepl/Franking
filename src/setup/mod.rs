/*! Setup wizard module wiring. */

mod accounts;
mod config;
mod discovered;
mod oauth_setup;
mod wizard;

#[cfg(test)]
mod tests;

pub use config::{secret_service_id, store_secret};
pub use wizard::{print_account_status, run_wizard};
