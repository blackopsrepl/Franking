/*! Worker module wiring. */

mod batch_ops;
mod dispatch;
mod folder_ops;
mod folders;
mod inbox;
mod oauth_ops;
mod outbox_ops;
mod sieve_ops;
mod template_ops;
mod watch;

pub use dispatch::{Worker, WorkerResult};
