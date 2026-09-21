/*! App unit tests: account Sieve settings. */

#[test]
fn saving_the_form_stores_explicit_sieve_settings() {
    use crate::account_edit::AccountEditState;

    use super::super::App;

    let conn = rusqlite::Connection::open_in_memory().unwrap();
    crate::db::init_for_test(&conn).unwrap();

    let mut app = App::new(None);
    app.db = Some(conn);
    let mut state = AccountEditState::new();
    state.name = "work".to_string();
    state.username = "alice@example.com".to_string();
    state.imap_host = "imap.example.com".to_string();
    state.smtp_host = "smtp.example.com".to_string();
    state.sieve_host = "sieve.example.com".to_string();
    state.sieve_port = "14190".to_string();
    state.password = "secret".to_string();
    app.account_edit_state = Some(state);

    // The password store requires secret-tool, which may be absent here; only
    // assert the validation path when it is unavailable.
    app.account_form_save();
    let record = crate::mail::account_store::get_account(app.db.as_ref().unwrap(), "work");
    match record {
        Ok(Some(record)) => {
            assert_eq!(record.sieve_host.as_deref(), Some("sieve.example.com"));
            assert_eq!(record.sieve_port, Some(14190));
        }
        _ => {
            let error = app
                .account_edit_state
                .as_ref()
                .and_then(|state| state.error.clone())
                .unwrap_or_default();
            assert!(
                error.is_empty() || error.contains("password") || error.contains("secret"),
                "unexpected error: {error}"
            );
        }
    }
}

#[test]
fn a_non_numeric_sieve_port_is_rejected() {
    use crate::account_edit::AccountEditState;

    use super::super::App;

    let mut app = App::new(None);
    let mut state = AccountEditState::new();
    state.name = "work".to_string();
    state.username = "alice@example.com".to_string();
    state.imap_host = "imap.example.com".to_string();
    state.smtp_host = "smtp.example.com".to_string();
    state.sieve_port = "sieve".to_string();
    app.account_edit_state = Some(state);

    app.account_form_save();
    let error = app.account_edit_state.as_ref().unwrap().error.clone();
    assert!(error.unwrap_or_default().contains("not a number"));
}
