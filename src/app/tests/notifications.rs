/*! Notifications follow receiving-account sender decisions. */

use crate::app::notification_rules::NotificationRule;
use crate::app::App;
use crate::db::sender_routes::{self, Route};

use super::triage::envelope;

#[test]
fn notification_rule_round_trips() {
    use crate::keys::View;
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    crate::db::init_for_test(&conn).unwrap();
    let mut app = App::new(None);
    app.db = Some(conn);
    assert_eq!(app.notification_rule, NotificationRule::Focused);
    app.open_settings();
    assert_eq!(app.view, View::Settings);
    for (rule, label) in [
        (NotificationRule::All, "every message"),
        (NotificationRule::Contacts, "contacts only"),
        (NotificationRule::Off, "off"),
        (NotificationRule::Focused, "focused inbox"),
    ] {
        app.cycle_notification_rule();
        assert_eq!(app.notification_rule, rule);
        assert!(app.status_message.contains(label));
    }
    let mut reloaded = App::new(None);
    let conn = app.db.take().unwrap();
    crate::db::preferences::set_text(&conn, "notification_rule", "contacts").unwrap();
    reloaded.db = Some(conn);
    reloaded.load_preferences();
    assert_eq!(reloaded.notification_rule, NotificationRule::Contacts);
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
fn focused_notifications_ignore_screening_reading_and_other_accounts() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    crate::db::init_for_test(&conn).unwrap();
    sender_routes::set(&conn, "work", "alice@example.org", Route::Inbox).unwrap();
    sender_routes::set(&conn, "personal", "alice@example.org", Route::Reading).unwrap();
    let mut app = App::new(Some("work".into()));
    app.db = Some(conn);
    assert_eq!(app.notification_rule, NotificationRule::Focused);
    let work = envelope("work", "1", "alice@example.org");
    let personal = envelope("personal", "2", "alice@example.org");
    let unknown = envelope("work", "3", "new@example.org");
    assert!(app.notify_for_new_mail(Some("work"), "INBOX", Some(&work)));
    assert!(!app.notify_for_new_mail(Some("personal"), "INBOX", Some(&personal)));
    assert!(!app.notify_for_new_mail(Some("work"), "INBOX", Some(&unknown)));
    assert!(!app.notify_for_new_mail(Some("work"), "Archive", Some(&work)));
    assert!(!app.notify_for_new_mail(Some("work"), "INBOX", None));
    let mut no_account = work.clone();
    no_account.account = None;
    assert!(!app.notify_for_new_mail(None, "INBOX", Some(&no_account)));
    app.notification_rule = NotificationRule::All;
    assert!(app.notify_for_new_mail(Some("work"), "INBOX", Some(&unknown)));
}
