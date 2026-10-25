/*! Notifications follow explicit preferences, not retired sender policies. */

use crate::app::notification_rules::NotificationRule;
use crate::app::App;

use super::envelope;

#[test]
fn notification_rule_round_trips() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    crate::db::init_for_test(&conn).unwrap();
    let mut app = App::new(None);
    app.db = Some(conn);
    assert_eq!(app.notification_rule, NotificationRule::All);
    for (rule, label) in [
        (NotificationRule::Contacts, "contacts only"),
        (NotificationRule::Off, "off"),
        (NotificationRule::All, "all inbox mail"),
    ] {
        app.cycle_notification_rule();
        assert_eq!(app.notification_rule, rule);
        assert!(app.status_message.contains(label));
    }
    let mut reloaded = App::new(None);
    reloaded.db = app.db.take();
    for (stored, expected) in [
        ("contacts", NotificationRule::Contacts),
        ("off", NotificationRule::Off),
        ("focused", NotificationRule::All),
        ("all", NotificationRule::All),
    ] {
        crate::db::preferences::set_text(
            reloaded.db.as_ref().unwrap(),
            "notification_rule",
            stored,
        )
        .unwrap();
        reloaded.load_preferences();
        assert_eq!(reloaded.notification_rule, expected);
    }
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    crate::db::init_for_test(&conn).unwrap();
    crate::db::preferences::set(&conn, "notifications", false).unwrap();
    reloaded.db = Some(conn);
    reloaded.load_preferences();
    assert_eq!(reloaded.notification_rule, NotificationRule::Off);
}

#[test]
fn every_inbox_message_notifies_by_default() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    crate::db::init_for_test(&conn).unwrap();
    let mut app = App::new(Some("work".into()));
    app.db = Some(conn);
    let work = envelope("work", "1", "alice@example.org");
    let personal = envelope("personal", "2", "alice@example.org");
    let unknown = envelope("work", "3", "new@example.org");
    for mail in [&work, &personal, &unknown] {
        assert!(app.notify_for_new_mail(None, "inbox", Some(mail)));
    }
    assert!(!app.notify_for_new_mail(Some("work"), "Archive", Some(&work)));
    assert!(app.notify_for_new_mail(Some("work"), "INBOX", None));
    let mut no_account = work.clone();
    no_account.account = None;
    assert!(app.notify_for_new_mail(None, "INBOX", Some(&no_account)));
}

#[test]
fn mute_and_off_silence_notifications() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    crate::db::init_for_test(&conn).unwrap();
    let mut app = App::new(Some("work".into()));
    let mail = envelope("work", "1", "new@example.org");
    let anchors = crate::db::conversations::anchors(&mail);
    crate::db::conversations::set_muted(&conn, "work", &anchors, true).unwrap();
    app.db = Some(conn);
    assert!(!app.notify_for_new_mail(Some("work"), "INBOX", Some(&mail)));
    assert!(app.notify_for_new_mail(
        Some("personal"),
        "INBOX",
        Some(&envelope("personal", "1", "new@example.org"))
    ));
    crate::db::conversations::set_muted(app.db.as_ref().unwrap(), "work", &anchors, false).unwrap();
    app.notification_rule = NotificationRule::Off;
    assert!(!app.notify_for_new_mail(Some("work"), "INBOX", Some(&mail)));
    assert!(!app.notify_for_new_mail(Some("work"), "INBOX", None));
    app.notification_rule = NotificationRule::Contacts;
    assert!(!app.notify_for_new_mail(Some("work"), "INBOX", Some(&mail)));
    assert!(!app.notify_for_new_mail(Some("work"), "INBOX", None));
    crate::contacts::upsert_harvested(app.db.as_ref().unwrap(), None, "new@example.org").unwrap();
    assert!(app.notify_for_new_mail(Some("work"), "INBOX", Some(&mail)));
}
