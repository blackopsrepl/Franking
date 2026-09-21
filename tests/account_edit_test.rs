/*! Account form state tests. */

use solverforge_mail::account_edit::{AccountEditState, AccountField};
use solverforge_mail::mail::account_store::AccountRecord;

#[test]
fn tab_cycles_through_every_field_and_wraps() {
    let mut state = AccountEditState::new();
    assert_eq!(state.focused, AccountField::Name);
    for _ in 0..12 {
        state.focused = state.focused.step(1);
    }
    assert_eq!(state.focused, AccountField::Cancel);
    state.focused = state.focused.step(1);
    assert_eq!(state.focused, AccountField::Name);
    state.focused = state.focused.step(-1);
    assert_eq!(state.focused, AccountField::Cancel);
}

#[test]
fn validation_requires_hosts_and_numeric_ports() {
    let mut state = AccountEditState::new();
    assert!(state.validate().is_err(), "empty form is invalid");

    state.name = "work".to_string();
    state.username = "alice@example.com".to_string();
    state.imap_host = "imap.example.com".to_string();
    state.smtp_host = "smtp.example.com".to_string();
    let (name, username, imap_host, imap_port, smtp_host, smtp_port) = state.validate().unwrap();
    assert_eq!(name, "work");
    assert_eq!(username, "alice@example.com");
    assert_eq!(imap_host, "imap.example.com");
    assert_eq!(smtp_host, "smtp.example.com");
    assert_eq!(imap_port, 993, "default IMAP port");
    assert_eq!(smtp_port, 465, "default SMTP port");

    state.imap_port = "not-a-port".to_string();
    assert!(state.validate().is_err());

    state.imap_port = "993".to_string();
    state.name = "with space".to_string();
    assert!(
        state.validate().is_err(),
        "account names must not have spaces"
    );
}

#[test]
fn editing_prefills_from_a_record_and_masks_the_password() {
    let record = AccountRecord {
        name: "work".to_string(),
        backend_kind: "imap".to_string(),
        provider_kind: "generic".to_string(),
        enabled: true,
        is_default: true,
        maildir_path: None,
        imap_host: Some("imap.example.com".to_string()),
        imap_port: Some(1993),
        imap_security: Some("tls".to_string()),
        smtp_host: Some("smtp.example.com".to_string()),
        smtp_port: Some(1465),
        smtp_security: Some("tls".to_string()),
        auth_mode: Some("password".to_string()),
        username: Some("alice@example.com".to_string()),
        keyring_imap_secret_id: Some("work-imap".to_string()),
        keyring_smtp_secret_id: Some("work-smtp".to_string()),
    };

    let mut state = AccountEditState::from_record(&record);
    assert!(state.editing);
    assert_eq!(state.name, "work");
    assert_eq!(state.imap_port, "1993");
    assert_eq!(state.smtp_port, "1465");
    assert!(state.is_default);
    assert!(state.password.is_empty());

    state.focused = AccountField::Password;
    state.focused_field_mut().unwrap().push_str("secret");
    assert_eq!(state.password, "secret");
    state.toggle_default();
    assert!(!state.is_default);
}
