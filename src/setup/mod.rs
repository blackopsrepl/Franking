/*! Setup wizard module wiring. */

mod accounts;
mod config;
mod discovered;
mod oauth_setup;
mod wizard;

#[cfg(test)]
mod tests;

pub use config::{
    load_account_record, load_oauth_state, lookup_secret, save_account_config, save_oauth_state,
    secret_service_id, store_secret,
};
pub use wizard::{print_account_status, run_wizard};
