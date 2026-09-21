/*! Account store module wiring. */

mod model;
mod oauth_store;
mod store;

#[cfg(test)]
mod tests;

pub use model::{AccountConfig, AccountRecord, OauthState, OauthStateConfig, TEST_ACCOUNT_NAME};
pub use oauth_store::{get_oauth_state, upsert_oauth_state};
pub use store::{
    delete_account, get_account, list_accounts, preferred_account, seed_defaults,
    set_default_account, upsert_account,
};
