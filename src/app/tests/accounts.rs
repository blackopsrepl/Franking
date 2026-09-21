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
