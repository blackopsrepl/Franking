/*! Worker module wiring. */

mod batch_ops;
mod dispatch;
mod folder_ops;
mod folders;
mod inbox;
mod sieve_ops;
mod template_ops;
mod watch;

pub use dispatch::{Worker, WorkerResult};
