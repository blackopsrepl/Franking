/*! Database module wiring. */

mod connection;
mod migration_v1_v2;
mod migration_v2_v3;
mod migration_v4_v5;
mod migration_v5_v6;
mod migration_v6_v7;
mod migration_v7_v8;
mod schema;

pub mod annotations;
pub mod conversations;
pub mod message_markers;
pub mod message_routes;
pub mod preferences;
pub mod saved_searches;
mod schema_migrations;
pub mod sender_routes;

pub use connection::{db_path, init_for_test, open, schema_version, CURRENT_SCHEMA_VERSION};

#[cfg(test)]
mod migration_history;
#[cfg(test)]
mod migration_tests;
