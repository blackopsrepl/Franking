/*! Legacy policy storage remains compatible but no longer hides server mail. */

use crate::app::App;
use crate::db::{account_policy, sender_routes};
use crate::keys::View;

use super::triage::envelope;

#[test]
fn a_bypass_token_round_trips_through_the_prompt() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    crate::db::init_for_test(&conn).unwrap();
    let mut app = App::new(Some("work".into()));
    app.db = Some(conn);
    app.refresh_bypass_token();
    assert!(app.bypass_token.is_none());
    app.open_bypass_prompt();
    assert_eq!(app.view, View::BypassPrompt);
    for c in "Red87".chars() {
        app.bypass_input(c);
    }
    app.submit_bypass();
    assert_eq!(app.bypass_token.as_deref(), Some("Red87"));
    assert_eq!(
        account_policy::bypass_token(app.db.as_ref().unwrap(), "work")
            .unwrap()
            .as_deref(),
        Some("Red87")
    );
}

#[test]
fn legacy_sender_routes_and_tokens_do_not_hide_inbox_mail() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    crate::db::init_for_test(&conn).unwrap();
    account_policy::set_bypass_token(&conn, "work", Some("Red87")).unwrap();
    // Legacy blocked routes remain stored but do not hide server mail.
    sender_routes::set(
        &conn,
        "work",
        "knock@example.org",
        sender_routes::Route::Blocked,
    )
    .unwrap();
    let mut app = App::new(Some("work".into()));
    app.db = Some(conn);
    app.refresh_bypass_token();
    app.triage_lane = Some(sender_routes::Route::Inbox);
    let mut unknown = envelope("work", "1", "stranger@example.org");
    unknown.subject = "Hello Red87 please".into();
    app.handle_envelopes_loaded(vec![unknown]);
    assert_eq!(app.envelopes.len(), 1);
    app.handle_envelopes_loaded(vec![envelope("work", "3", "unknown@example.org")]);
    assert_eq!(
        app.envelopes.len(),
        1,
        "unknown senders need no bypass token"
    );

    let mut blocked = envelope("work", "2", "knock@example.org");
    blocked.subject = "Hello Red87 please".into();
    app.handle_envelopes_loaded(vec![blocked]);
    assert_eq!(
        app.envelopes.len(),
        1,
        "local blocks do not hide server mail"
    );
}

#[test]
fn a_legacy_cover_preference_does_not_hide_seen_mail() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    crate::db::init_for_test(&conn).unwrap();
    sender_routes::set(
        &conn,
        "work",
        "alice@example.org",
        sender_routes::Route::Inbox,
    )
    .unwrap();
    let mut app = App::new(Some("work".into()));
    app.db = Some(conn);
    app.triage_lane = Some(sender_routes::Route::Inbox);
    crate::db::preferences::set(app.db.as_ref().unwrap(), "cover_seen", true).unwrap();
    app.load_preferences();
    let mut seen = envelope("work", "1", "alice@example.org");
    seen.flags.push("seen".into());
    let unseen = envelope("work", "2", "alice@example.org");
    app.handle_envelopes_loaded(vec![seen, unseen]);
    assert_eq!(app.envelopes.len(), 2);
    assert_eq!(app.covered_count, 0);
    app.toggle_cover_reveal();
    assert!(app.cover_revealed);
}
