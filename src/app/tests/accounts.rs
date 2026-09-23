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

#[test]
fn the_form_stores_the_chosen_connection_security() {
    use crate::account_edit::{AccountEditState, ConnectionSecurity};
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
    state.imap_port = "1153".to_string();
    state.smtp_host = "127.0.0.1".to_string();
    state.smtp_port = "1025".to_string();
    // Editing an existing account, so no password is needed and no keyring
    // lookup happens; the security choice is what this covers.
    state.editing = true;
    state.imap_security = ConnectionSecurity::Plain;
    state.smtp_security = ConnectionSecurity::StartTls;
    app.account_edit_state = Some(state);

    app.account_form_save();

    let stored = crate::mail::account_store::list_accounts(app.db.as_ref().unwrap())
        .unwrap()
        .into_iter()
        .find(|account| account.name == "dovecot")
        .expect("the account is stored");
    assert_eq!(stored.imap_security.as_deref(), Some("plain"));
    assert_eq!(stored.smtp_security.as_deref(), Some("starttls"));
}

#[test]
fn typing_a_port_moves_the_security_with_it() {
    use crate::account_edit::{AccountEditState, AccountField, ConnectionSecurity};

    let mut state = AccountEditState::new();
    state.focused = AccountField::ImapPort;
    state.imap_port.clear();
    for c in "1153".chars() {
        state.imap_port.push(c);
        state.sync_security_to_ports();
    }
    assert_eq!(
        state.imap_security,
        ConnectionSecurity::StartTls,
        "a cleartext port selects STARTTLS"
    );
}
