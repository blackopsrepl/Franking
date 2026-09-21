/*! IMAP session module wiring. */

mod capabilities;
mod client;
mod client_connect;
mod codec;
mod credentials;
mod errors;
mod idle;
mod pool;
mod security;
mod transport;

#[cfg(test)]
mod tests;

pub use capabilities::Capabilities;
pub use client::{tag_for, ImapClient};
pub use client_connect::open_imap_client;
pub use codec::{CommandOutput, Completion, ResponseReader};
pub use credentials::{CredentialProvider, KeyringCredentials};
pub use errors::{looks_like_auth_failure, map_codec_error};
pub use idle::{wait as idle_wait, IdleOutcome};
pub use pool::SessionPool;
pub use security::Security;
pub use transport::ReadWrite;
