/*! IMAP session module wiring. */

mod capabilities;
mod client;
mod client_connect;
mod codec;
mod connection;
mod credentials;
mod pool;
mod transport;

#[cfg(test)]
mod tests;

pub use capabilities::Capabilities;
pub use client::{tag_for, ImapClient};
pub use client_connect::open_imap_client;
pub use codec::{CommandOutput, Completion, ResponseReader};
pub use connection::{
    connect, looks_like_auth_failure, map_imap_error, probe_capabilities, ConnectedImapSession,
    IdleOutcome, ImapSession, Security,
};
pub use credentials::{CredentialProvider, KeyringCredentials};
pub use pool::SessionPool;
pub use transport::ReadWrite;
