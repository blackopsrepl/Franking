/*! Worker module wiring. */

mod dispatch;
mod inbox;
mod watch;

pub use dispatch::{Worker, WorkerResult};
