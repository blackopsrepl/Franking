/*! OAuth unit tests. */

use super::authorize::code_challenge;
use super::refresh::expires_at_rfc3339;
use super::{provider_by_kind, GMAIL_PROVIDER};

#[test]
fn providers_are_registered_by_kind() {
    assert_eq!(
        provider_by_kind("gmail").map(|provider| provider.display_name),
        Some("Gmail")
    );
    assert_eq!(
        provider_by_kind("outlook").map(|provider| provider.display_name),
        Some("Outlook")
    );
    assert!(provider_by_kind("unknown").is_none());
}

#[test]
fn code_challenge_is_urlsafe() {
    let challenge = code_challenge("test-verifier");
    assert!(!challenge.contains('='));
    assert!(!challenge.contains('+'));
    assert!(!challenge.contains('/'));
}

#[test]
fn expires_at_is_generated_for_token_lifetime() {
    let expires_at = expires_at_rfc3339(Some(3600)).unwrap();
    assert!(expires_at.contains('T'));
    assert!(expires_at.contains('+') || expires_at.ends_with('Z'));
}

#[test]
fn gmail_provider_uses_mail_scope() {
    assert_eq!(GMAIL_PROVIDER.scopes, &["https://mail.google.com/"]);
}
