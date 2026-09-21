//! Delta synchronization over CONDSTORE and QRESYNC.

use std::sync::Arc;

use solverforge_mail::mail::remote::next;
use solverforge_mail::mail::session::{open_imap_client, SessionPool};

use super::support::{
    account, ensure_mailbox, mailbox_lock, seed, test_address, FixedCredentials, FOLDER,
};

/// Delta sync must report removed messages and flag changes, and must never
/// silently claim "nothing changed" when the anchor cannot be used.
#[test]
fn qresync_reports_vanished_and_changed_flags() {
    let Some((host, port)) = test_address() else {
        return;
    };
    let _guard = mailbox_lock();
    let account = account(&host, port);
    ensure_mailbox(&account);
    seed(&account);

    let pool = Arc::new(SessionPool::with_credentials(Arc::new(FixedCredentials)));
    let service = solverforge_mail::mail::remote::ImapSmtpService::new(account.clone(), pool);

    // First sync: no anchor, so a full listing and an anchor come back.
    let first = service
        .sync_folder_delta(None, FOLDER, None)
        .expect("initial sync");
    assert!(
        first.full_resync,
        "the first sync has nothing to resume from"
    );
    let uid_validity = first.uid_validity.expect("UIDVALIDITY");
    let highest_modseq = first.highest_modseq.expect("HIGHESTMODSEQ");
    assert!(highest_modseq > 0);
    assert!(first.uid_next.expect("UIDNEXT") > 0);

    let uid_next = first.uid_next.unwrap();
    let anchor = solverforge_mail::mail::remote::next::SyncAnchor {
        uid_validity,
        highest_modseq,
    };

    // Append one message and flag another, then ask for the delta.
    let fresh = format!(
        "From: bob@example.com\r\nTo: test@example.com\r\nSubject: After the anchor\r\nMessage-ID: <after-{uid_next}@example.com>\r\nDate: 2026-04-13 10:00:00+00:00\r\n\r\nnew mail"
    );
    let mut client = open_imap_client(&account, &FixedCredentials).expect("client");
    let appended = next::append(&mut client, FOLDER, vec![], fresh.as_bytes())
        .expect("append")
        .expect("APPENDUID");
    next::select(&mut client, FOLDER).expect("select");
    let seen = next::flag_of("seen").expect("flag");
    next::store_flags(
        &mut client,
        &appended.uid.to_string(),
        next::FlagChange::Add,
        vec![seen],
        true,
    )
    .expect("store");
    client.logout();

    let delta = service
        .sync_folder_delta(None, FOLDER, Some(anchor))
        .expect("delta sync");
    assert!(!delta.full_resync, "the anchor was usable");
    assert!(
        delta
            .changed_flags
            .iter()
            .any(|(uid, flags)| *uid == appended.uid
                && flags.iter().any(|flag| flag.eq_ignore_ascii_case("seen"))),
        "the flag change is reported: {:?}",
        delta.changed_flags
    );
    assert!(
        delta.highest_modseq.expect("new modseq") > highest_modseq,
        "the anchor advanced"
    );

    // A vanished message must be reported as removed.
    let mut client = open_imap_client(&account, &FixedCredentials).expect("client");
    next::select(&mut client, FOLDER).expect("select");
    let doomed = next::search_uids(&mut client, next::search::criteria(None))
        .expect("search")
        .into_iter()
        .filter(|uid| *uid != appended.uid)
        .min()
        .expect("another message to delete");
    let deleted = next::flag_of("deleted").expect("flag");
    next::store_flags(
        &mut client,
        &doomed.to_string(),
        next::FlagChange::Add,
        vec![deleted],
        true,
    )
    .expect("store");
    next::expunge_uids(&mut client, &doomed.to_string()).expect("expunge");
    client.logout();

    let delta = service
        .sync_folder_delta(None, FOLDER, Some(anchor))
        .expect("delta sync after expunge");
    assert!(!delta.full_resync, "the anchor is still usable");
    assert!(
        delta.vanished.contains(&doomed),
        "the expunged UID {doomed} is reported as vanished: {:?}",
        delta.vanished
    );
}
