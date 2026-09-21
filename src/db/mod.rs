/*! Database module wiring. */

mod connection;
mod schema;

pub mod preferences;

pub use connection::{db_path, init_for_test, open, schema_version, CURRENT_SCHEMA_VERSION};
