/*! IMAP session module wiring. */

mod capabilities;
mod codec;
mod connection;
mod credentials;
mod pool;

#[cfg(test)]
mod tests;

pub use capabilities::Capabilities;
pub use codec::{CommandOutput, Completion, ResponseReader};
pub use connection::{
    connect, looks_like_auth_failure, map_imap_error, probe_capabilities, ConnectedImapSession,
    IdleOutcome, ImapSession, Security,
};
pub use credentials::{CredentialProvider, KeyringCredentials};
pub use pool::SessionPool;
