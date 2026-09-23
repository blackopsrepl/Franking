/*! Database module wiring. */

mod connection;
mod migration_v1_v2;
mod migration_v2_v3;
mod schema;

pub mod preferences;
pub mod saved_searches;
mod schema_migrations;
pub mod sender_routes;

pub use connection::{db_path, init_for_test, open, schema_version, CURRENT_SCHEMA_VERSION};

#[cfg(test)]
mod migration_tests;
