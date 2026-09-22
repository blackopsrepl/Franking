/*! App unit tests: account discovery. */

use crate::app::App;

#[test]
fn discovered_settings_fill_the_account_form() {
    use crate::account_edit::AccountEditState;
    use crate::mail::autoconfig::{DiscoveredConfig, DiscoverySource};

    let mut app = App::new(None);
    app.account_edit_state = Some(AccountEditState::new());

    app.apply_discovered(Some(DiscoveredConfig {
        provider_kind: "gmail".to_string(),
        imap_host: "imap.gmail.com".to_string(),
        imap_port: 993,
        imap_security: "tls".to_string(),
        smtp_host: "smtp.gmail.com".to_string(),
        smtp_port: 465,
        smtp_security: "tls".to_string(),
        username: "alice@gmail.com".to_string(),
        auth_mode: "oauth2".to_string(),
        source: DiscoverySource::Preset,
    }));

    let state = app.account_edit_state.as_ref().expect("form");
    assert_eq!(state.imap_host, "imap.gmail.com");
    assert_eq!(state.imap_port, "993");
    assert_eq!(state.smtp_host, "smtp.gmail.com");
    assert_eq!(state.smtp_port, "465");
    assert_eq!(state.username, "alice@gmail.com");
    assert!(app.status_message.contains("built-in provider settings"));

    app.apply_discovered(None);
    assert!(app.status_message.contains("No automatic settings found"));
}

#[test]
fn oauth_mode_requires_a_client_id_and_an_email() {
    use crate::account_edit::{AccountEditState, AccountField, AuthMode};

    let mut app = App::new(None);
    let mut state = AccountEditState::new();
    state.name = "work".to_string();
    state.username = "alice@gmail.com".to_string();
    state.imap_host = "imap.gmail.com".to_string();
    state.smtp_host = "smtp.gmail.com".to_string();
    state.auth_mode = AuthMode::GmailOAuth;
    app.account_edit_state = Some(state);

    app.account_form_save();
    let error = app.account_edit_state.as_ref().unwrap().error.clone();
    assert!(error.unwrap_or_default().contains("client ID"));

    // A client id without an email address is still rejected.
    let state = app.account_edit_state.as_mut().unwrap();
    state.client_id = "client-123".to_string();
    state.username = "not-an-email".to_string();
    state.focused = AccountField::Save;
    app.account_form_save();
    let error = app.account_edit_state.as_ref().unwrap().error.clone();
    assert!(error.unwrap_or_default().contains("email address"));
}

#[test]
fn space_cycles_the_auth_mode_when_focused() {
    use crate::account_edit::{AccountEditState, AccountField, AuthMode};

    let mut app = App::new(None);
    let mut state = AccountEditState::new();
    state.focused = AccountField::Auth;
    app.account_edit_state = Some(state);

    app.account_form_toggle_default();
    assert_eq!(
        app.account_edit_state.as_ref().unwrap().auth_mode,
        AuthMode::GmailOAuth
    );
    app.account_form_toggle_default();
    assert_eq!(
        app.account_edit_state.as_ref().unwrap().auth_mode,
        AuthMode::OutlookOAuth
    );
    app.account_form_toggle_default();
    assert_eq!(
        app.account_edit_state.as_ref().unwrap().auth_mode,
        AuthMode::Password
    );
}

#[test]
fn discovery_requires_an_email_address() {
    use crate::account_edit::AccountEditState;

    let mut app = App::new(None);
    app.account_edit_state = Some(AccountEditState::new());

    app.discover_account_settings();
    let error = app.account_edit_state.as_ref().unwrap().error.clone();
    assert!(error.unwrap_or_default().contains("email address"));
    assert!(!app.loading, "invalid input must not start a lookup");
}

#[test]
fn notification_rule_round_trips() {
    use crate::app::notification_rules::NotificationRule;
    use crate::keys::View;

    use super::super::App;

    let conn = rusqlite::Connection::open_in_memory().unwrap();
    crate::db::init_for_test(&conn).unwrap();

    let mut app = App::new(None);
    app.db = Some(conn);
    assert_eq!(app.notification_rule, NotificationRule::All, "the default");

    app.open_settings();
    assert_eq!(app.view, View::Settings);

    // Off, every message, contacts only, then back to off.
    app.cycle_notification_rule();
    assert_eq!(app.notification_rule, NotificationRule::Contacts);
    assert!(app.status_message.contains("contacts only"));
    app.cycle_notification_rule();
    assert_eq!(app.notification_rule, NotificationRule::Off);
    assert!(app.status_message.contains("off"));
    app.cycle_notification_rule();
    assert_eq!(app.notification_rule, NotificationRule::All);

    // A fresh app picks the stored rule back up.
    let mut reloaded = App::new(None);
    let conn = app.db.take().unwrap();
    crate::db::preferences::set_text(&conn, "notification_rule", "contacts").unwrap();
    reloaded.db = Some(conn);
    reloaded.load_preferences();
    assert_eq!(reloaded.notification_rule, NotificationRule::Contacts);

    // An install that only has the older boolean keeps its choice.
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    crate::db::init_for_test(&conn).unwrap();
    crate::db::preferences::set(&conn, "notifications", false).unwrap();
    let mut older = App::new(None);
    older.db = Some(conn);
    older.load_preferences();
    assert_eq!(older.notification_rule, NotificationRule::Off);

    app.close_settings();
    assert_eq!(app.view, View::EnvelopeList);
}

#[test]
fn contact_tag_filter_cycles_through_tags_and_back_to_all() {
    use crate::contacts::{self, Contact};

    use super::super::App;

    let conn = rusqlite::Connection::open_in_memory().unwrap();
    crate::db::init_for_test(&conn).unwrap();
    let alice = contacts::add(
        &conn,
        &Contact {
            id: 0,
            name: Some("Alice".to_string()),
            email: "alice@example.com".to_string(),
            phone: None,
            org: None,
            notes: None,
            harvested: false,
            tags: Vec::new(),
        },
    )
    .unwrap();
    let bob = contacts::add(
        &conn,
        &Contact {
            id: 0,
            name: Some("Bob".to_string()),
            email: "bob@example.com".to_string(),
            phone: None,
            org: None,
            notes: None,
            harvested: false,
            tags: Vec::new(),
        },
    )
    .unwrap();
    contacts::add_tag(&conn, alice, "work").unwrap();
    contacts::add_tag(&conn, bob, "family").unwrap();

    let mut app = App::new(None);
    app.db = Some(conn);
    app.open_contacts();
    assert_eq!(app.contacts.len(), 2);

    app.cycle_contact_tag();
    assert_eq!(app.contact_tag_filter.as_deref(), Some("family"));
    assert_eq!(app.contacts.len(), 1);
    assert_eq!(app.contacts[0].email, "bob@example.com");

    app.cycle_contact_tag();
    assert_eq!(app.contact_tag_filter.as_deref(), Some("work"));
    assert_eq!(app.contacts[0].email, "alice@example.com");

    app.cycle_contact_tag();
    assert!(app.contact_tag_filter.is_none());
    assert_eq!(app.contacts.len(), 2);
}

#[test]
fn saving_an_incomplete_account_reports_what_is_missing() {
    use crate::account_edit::AccountEditState;
    use crate::keys::View;

    use super::super::App;

    let conn = rusqlite::Connection::open_in_memory().unwrap();
    crate::db::init_for_test(&conn).unwrap();

    let mut app = App::new(None);
    app.db = Some(conn);
    app.view = View::AccountEdit;
    let mut state = AccountEditState::new();
    state.name = "dovecot".to_string();
    state.username = "test@localhost".to_string();
    state.imap_host = "127.0.0.1".to_string();
    // No SMTP host: the form must say so.
    app.account_edit_state = Some(state);

    app.account_form_save();

    let state = app
        .account_edit_state
        .as_ref()
        .expect("the form stays open");
    assert_eq!(
        state.error.as_deref(),
        Some("SMTP host is required."),
        "the missing field is named"
    );
    assert_eq!(app.view, View::AccountEdit, "the form is not closed");
}
