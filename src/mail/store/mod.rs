/*! Local message store module wiring. */

mod messages;
mod model;
mod queries;
mod sync;

#[cfg(test)]
mod tests;

pub use messages::{
    count_messages, delete_message, get_message, list_messages, retain_uids, search_messages,
    thread_messages, upsert_envelope, upsert_message,
};
pub use model::{StoredMessage, SyncState};
pub use sync::{get_sync_state, set_sync_state};
