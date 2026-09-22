//! Live verification of the app-owned IMAP command layer.
//!
//! Runs only when `FRANKING_IMAP_TEST_ADDR` points at a Dovecot test
//! container, per the recipe in `dovecot_test.rs`. It exercises exactly what
//! the legacy `imap` crate could not: SORT, THREAD, CONDSTORE and QRESYNC.

mod codec_tests;
mod delta_tests;
mod sort_tests;
mod support;
