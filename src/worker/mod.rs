/*! Worker module wiring. */

mod dispatch;
mod folder_ops;
mod folders;
mod inbox;
mod sieve_ops;
mod watch;

pub use dispatch::{Worker, WorkerResult};
