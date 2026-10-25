/*! Worker module wiring. */

mod attachment_ops;
mod batch_ops;
mod dispatch;
mod folder_ops;
mod folder_rules;
mod folders;
mod inbox;
mod listing;
mod oauth_ops;
mod outbox_ops;
mod outgoing_ops;
mod read_together_ops;
mod search_ops;
mod sieve_ops;
mod template_ops;
mod watch;

pub use dispatch::{Worker, WorkerResult};
pub(crate) use inbox::MAX_PER_ACCOUNT;
