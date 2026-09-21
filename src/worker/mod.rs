/*! Worker module wiring. */

mod dispatch;
mod folders;
mod inbox;
mod watch;

pub use dispatch::{Worker, WorkerResult};
