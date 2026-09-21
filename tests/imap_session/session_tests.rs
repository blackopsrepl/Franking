//! IMAP session integration tests.

use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use solverforge_mail::mail::session::{
    map_imap_error, ConnectedImapSession, IdleOutcome, SessionPool,
};

use super::fake_imap::{Behavior, FakeImap};
use super::support::{account, select_inbox, FixedCredentials};

#[test]
fn append_delivers_the_message_to_the_sent_mailbox() {
    let server = FakeImap::start(Behavior::Normal);
    let pool = SessionPool::with_credentials(Arc::new(FixedCredentials));
    let account = account(server.port);

    let payload: &[u8] = b"From: alice@example.com\r\nSubject: Hi\r\n\r\nhello";

    pool.with_connection(&account, |connection| match connection {
        ConnectedImapSession::Plain(session) => {
            session.append("Sent", payload).map_err(map_imap_error)
        }
        ConnectedImapSession::Tls(session) => {
            session.append("Sent", payload).map_err(map_imap_error)
        }
    })
    .unwrap();

    assert_eq!(server.appended.lock().unwrap().as_slice(), payload);
}

#[test]
fn session_pool_reuses_one_connection_across_operations() {
    let server = FakeImap::start(Behavior::Normal);
    let pool = SessionPool::with_credentials(Arc::new(FixedCredentials));
    let account = account(server.port);

    for _ in 0..2 {
        pool.with_connection(&account, select_inbox).unwrap();
    }

    assert_eq!(server.connections.load(Ordering::SeqCst), 1);
    assert_eq!(server.logins.load(Ordering::SeqCst), 1);
}

#[test]
fn capabilities_are_probed_on_connect() {
    let server = FakeImap::start(Behavior::Normal);
    let pool = SessionPool::with_credentials(Arc::new(FixedCredentials));
    let account = account(server.port);

    let capabilities = pool
        .with_connection(&account, |connection| {
            Ok(match connection {
                ConnectedImapSession::Plain(session) => {
                    let caps = session.capabilities().map_err(map_imap_error)?;
                    solverforge_mail::mail::session::Capabilities::from_imap(&caps)
                }
                ConnectedImapSession::Tls(session) => {
                    let caps = session.capabilities().map_err(map_imap_error)?;
                    solverforge_mail::mail::session::Capabilities::from_imap(&caps)
                }
            })
        })
        .unwrap();

    assert!(capabilities.idle);
    assert!(capabilities.move_);
    assert!(capabilities.uidplus);
    assert!(capabilities.imap4rev1);
}

#[test]
fn idle_watch_reports_mailbox_change() {
    let server = FakeImap::start(Behavior::Normal);
    let pool = SessionPool::with_credentials(Arc::new(FixedCredentials));
    let account = account(server.port);

    let outcome = pool
        .idle_wait(&account, "INBOX", Duration::from_secs(2))
        .unwrap();

    assert_eq!(outcome, IdleOutcome::MailboxChanged);
}

#[test]
fn transport_failure_is_retried_on_a_fresh_connection() {
    let server = FakeImap::start(Behavior::DropOnSelect);
    let pool = SessionPool::with_credentials(Arc::new(FixedCredentials));
    let account = account(server.port);

    // The first SELECT drops the connection; the pool must reconnect and retry.
    pool.with_connection(&account, select_inbox).unwrap();

    assert_eq!(server.connections.load(Ordering::SeqCst), 2);
    assert_eq!(server.logins.load(Ordering::SeqCst), 2);
}
