use super::router::sort_account_records;
use crate::mail::account_store::AccountRecord;

fn stored_account(name: &str, backend_kind: &str) -> AccountRecord {
    AccountRecord {
        name: name.to_string(),
        backend_kind: backend_kind.to_string(),
        provider_kind: "generic".to_string(),
        enabled: true,
        is_default: false,
        maildir_path: None,
        imap_host: Some("imap.example.com".to_string()),
        imap_port: Some(993),
        imap_security: Some("tls".to_string()),
        smtp_host: Some("smtp.example.com".to_string()),
        smtp_port: Some(465),
        smtp_security: Some("tls".to_string()),
        sieve_host: None,
        sieve_port: None,
        sieve_security: None,
        auth_mode: Some("password".to_string()),
        username: Some("alice@example.com".to_string()),
        keyring_imap_secret_id: Some("solverforge-mail/work/imap".to_string()),
        keyring_smtp_secret_id: Some("solverforge-mail/work/smtp".to_string()),
    }
}

#[test]
fn stored_imap_accounts_are_routable() {
    assert!(stored_account("work", "imap").is_routable());
    assert!(stored_account("test", "maildir").is_routable());
}

#[test]
fn account_sort_keeps_default_remote_ahead_of_local_test_account() {
    let mut accounts = vec![
        stored_account("test", "maildir"),
        AccountRecord {
            is_default: true,
            ..stored_account("work", "imap")
        },
    ];

    sort_account_records(&mut accounts);

    assert_eq!(accounts[0].name, "work");
    assert_eq!(accounts[1].name, "test");
}

#[test]
fn offline_errors_are_classified_for_cache_fallback() {
    use super::cache::is_offline;
    use crate::mail::MailError;

    assert!(is_offline(&MailError::transport_timeout("timed out")));
    assert!(is_offline(&MailError::connection_dropped("closed")));
    assert!(!is_offline(&MailError::imap_auth_rejected("bad password")));
    assert!(!is_offline(&MailError::config_invalid("missing host")));
}

#[test]
fn cached_envelopes_serve_listings_and_search_offline() {
    use super::cache::cached_envelopes;
    use crate::mail::store::{upsert_envelope, StoredMessage};
    use crate::mail::types::{Envelope, Sender};

    let conn = rusqlite::Connection::open_in_memory().unwrap();
    crate::db::init_for_test(&conn).unwrap();

    for (uid, subject) in [("1", "Quarterly report"), ("2", "Lunch plans")] {
        let envelope = Envelope {
            id: uid.to_string(),
            flags: Vec::new(),
            subject: subject.to_string(),
            sender: Sender::Plain("alice@example.com".to_string()),
            date: "2026-04-13 09:00:00+00:00".to_string(),
            message_id: None,
            in_reply_to: None,
            account: None,
            folder: None,
        };
        upsert_envelope(
            &conn,
            &StoredMessage::from_envelope("work", "INBOX", &envelope),
        )
        .unwrap();
    }

    let listed = cached_envelopes(&conn, "work", "INBOX", 1, 10, None).unwrap();
    assert_eq!(listed.len(), 2);

    let searched = cached_envelopes(&conn, "work", "INBOX", 1, 10, Some("lunch")).unwrap();
    assert_eq!(searched.len(), 1);
    assert_eq!(searched[0].id, "2");

    let other_folder = cached_envelopes(&conn, "work", "Sent", 1, 10, None).unwrap();
    assert!(other_folder.is_empty());
}
