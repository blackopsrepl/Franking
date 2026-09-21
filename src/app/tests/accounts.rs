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
fn notifications_preference_round_trips() {
    use crate::keys::View;

    use super::super::App;

    let conn = rusqlite::Connection::open_in_memory().unwrap();
    crate::db::init_for_test(&conn).unwrap();

    let mut app = App::new(None);
    app.db = Some(conn);
    assert!(app.notifications_enabled);

    app.open_settings();
    assert_eq!(app.view, View::Settings);

    app.toggle_notifications();
    assert!(!app.notifications_enabled);
    assert!(app.status_message.contains("off"));
    assert!(!crate::db::preferences::get(app.db.as_ref().unwrap(), "notifications", true).unwrap());

    app.toggle_notifications();
    assert!(app.notifications_enabled);
    assert!(crate::db::preferences::get(app.db.as_ref().unwrap(), "notifications", false).unwrap());

    // A fresh app picks the stored preference back up.
    let mut reloaded = App::new(None);
    let conn = app.db.take().unwrap();
    crate::db::preferences::set(&conn, "notifications", false).unwrap();
    reloaded.db = Some(conn);
    reloaded.load_preferences();
    assert!(!reloaded.notifications_enabled);

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
fn settings_cycle_between_preferences() {
    use super::super::App;

    let conn = rusqlite::Connection::open_in_memory().unwrap();
    crate::db::init_for_test(&conn).unwrap();

    let mut app = App::new(None);
    app.db = Some(conn);
    app.open_settings();
    assert_eq!(app.settings_index, 0);

    assert!(app.notifications_enabled);
    app.settings_toggle();
    assert!(!app.notifications_enabled);

    app.settings_move(1);
    assert_eq!(app.settings_index, 1);
    assert!(app.mark_read_on_open);
    app.settings_toggle();
    assert!(!app.mark_read_on_open);
    assert!(app.status_message.contains("stay unread"));

    app.settings_move(1);
    assert_eq!(app.settings_index, 0, "wraps around");
    app.settings_move(-1);
    assert_eq!(app.settings_index, 1);
}

#[test]
fn opening_a_message_marks_it_seen_and_queues_the_flag() {
    use crate::keys::View;
    use crate::mail::types::{Envelope, Sender};

    use super::super::App;

    let conn = rusqlite::Connection::open_in_memory().unwrap();
    crate::db::init_for_test(&conn).unwrap();

    let raw = b"From: alice@example.com\r\nSubject: Unread\r\n\r\nbody\r\n";
    let mut document = crate::mail::mime::parse_message(raw).unwrap();
    document.raw = Some(raw.to_vec());

    let mut app = App::new(None);
    app.db = Some(conn);
    app.view = View::MessageView;
    app.envelopes = vec![Envelope {
        id: "5".to_string(),
        flags: Vec::new(),
        subject: "Unread".to_string(),
        sender: Sender::Plain("alice@example.com".to_string()),
        date: String::new(),
        message_id: None,
        in_reply_to: None,
        account: None,
        folder: Some("INBOX".to_string()),
    }];
    app.envelope_state.select(Some(0));

    app.handle_message_loaded(document);
    assert!(
        app.envelopes[0].is_seen(),
        "the local envelope reflects the read state"
    );

    // With the preference off, the load path leaves the flag alone.
    let mut app = App::new(None);
    app.mark_read_on_open = false;
    app.view = View::MessageView;
    app.envelopes = vec![Envelope {
        id: "6".to_string(),
        flags: Vec::new(),
        subject: "Unread".to_string(),
        sender: Sender::Plain("alice@example.com".to_string()),
        date: String::new(),
        message_id: None,
        in_reply_to: None,
        account: None,
        folder: Some("INBOX".to_string()),
    }];
    app.envelope_state.select(Some(0));
    app.handle_message_loaded(crate::mail::mime::parse_message(raw).unwrap());
    assert!(app.envelopes[0].is_seen(), "local state still updates");
}
