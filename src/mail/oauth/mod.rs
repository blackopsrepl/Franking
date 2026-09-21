/*! OAuth module wiring. */

pub mod account;
mod authorize;
mod providers;
mod refresh;
mod token;

#[cfg(test)]
mod tests;

pub use authorize::authorize_account;
pub use providers::{
    provider_by_kind, OAuthAuthorization, OAuthProvider, OAuthProviderKind, GMAIL_PROVIDER,
    OUTLOOK_PROVIDER,
};
pub use refresh::ensure_access_token;
