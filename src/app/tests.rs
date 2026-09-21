/*! App unit tests. */

use crate::himalaya::diagnostics as himalaya_diagnostics;

#[test]
fn explain_himalaya_error_flags_maildir_as_non_auth() {
    let raw = "himalaya error: cannot open local maildir";
    let explained = himalaya_diagnostics::explain(Some("maildir"), raw);
    assert!(explained.contains("not an authentication error"));
}

#[test]
fn explain_himalaya_error_flags_keyring_failures() {
    let raw = "himalaya error: failed to talk to Secret Service keyring";
    let explained = himalaya_diagnostics::explain(Some("imap"), raw);
    assert!(explained.contains("Keyring secret missing or inaccessible"));
}

#[test]
fn explain_himalaya_error_flags_oauth_failures() {
    let raw = "himalaya error: invalid_grant while refreshing oauth token";
    let explained = himalaya_diagnostics::explain(Some("imap"), raw);
    assert!(explained.contains("OAuth credentials need reconfiguration"));
}
