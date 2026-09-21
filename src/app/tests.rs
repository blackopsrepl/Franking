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

#[test]
fn unlock_prompt_adopts_typed_passphrase() {
    use super::App;
    use crate::keys::View;

    let mut app = App::new(None);
    app.enter_unlock_prompt();
    assert_eq!(app.view, View::PassphrasePrompt);

    for c in "secret".chars() {
        app.unlock_input(c);
    }
    app.unlock_backspace();
    app.submit_unlock();

    assert_eq!(app.view, View::MessageView);
    assert_eq!(app.crypto_passphrase, "secre");
    assert!(app.unlock_input.is_empty());
}

#[test]
fn unlock_cancel_keeps_cached_passphrase() {
    use super::App;
    use crate::keys::View;

    let mut app = App::new(None);
    app.crypto_passphrase = "cached".to_string();
    app.enter_unlock_prompt();
    app.unlock_input('x');
    app.cancel_unlock();

    assert_eq!(app.view, View::MessageView);
    assert!(app.unlock_input.is_empty());
    assert_eq!(app.crypto_passphrase, "cached");
}

#[test]
fn compose_toggles_signing_and_encryption() {
    use crate::compose::{ComposeMode, ComposeState, FocusedField};

    use super::App;

    let mut app = App::new(None);
    app.compose_state = Some(ComposeState::new(ComposeMode::New, None));

    let focused = |app: &mut App, field: FocusedField| {
        app.compose_state.as_mut().expect("compose").focused = field;
        app.compose_enter_insert();
    };

    focused(&mut app, FocusedField::Sign);
    assert!(app.compose_state.as_ref().expect("compose").sign);

    focused(&mut app, FocusedField::Encrypt);
    assert!(app.compose_state.as_ref().expect("compose").encrypt);
    focused(&mut app, FocusedField::Encrypt);
    assert!(!app.compose_state.as_ref().expect("compose").encrypt);
}
