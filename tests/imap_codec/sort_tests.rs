//! Server-side ordering of a paged folder listing.

use std::sync::Arc;

use solverforge_mail::mail::remote::next;
use solverforge_mail::mail::remote::ImapSmtpService;
use solverforge_mail::mail::session::{open_imap_client, SessionPool};
use solverforge_mail::mail::sort::{SortKey, SortOrder};

use super::support::{
    account, ensure_mailbox, mailbox_lock, test_address, FixedCredentials, FOLDER,
};

/// SORT must order the whole folder, not just the fetched page.
#[test]
fn sorted_listing_is_ordered_across_pages() {
    let Some((host, port)) = test_address() else {
        return;
    };
    let _guard = mailbox_lock();
    let account = account(&host, port);
    ensure_mailbox(&account);

    let mut client = open_imap_client(&account, &FixedCredentials).expect("client");
    // The mailbox is shared between runs, so start from a known state.
    next::select(&mut client, FOLDER).expect("select");
    let leftovers = next::search_uids(&mut client, next::search::criteria(Some("subject Sorted")))
        .expect("search leftovers");
    if !leftovers.is_empty() {
        let deleted = next::flag_of("deleted").expect("flag");
        let set = leftovers
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(",");
        next::store_flags(
            &mut client,
            &set,
            next::FlagChange::Add,
            vec![deleted],
            true,
        )
        .expect("store");
        next::expunge_uids(&mut client, &set).expect("expunge");
    }

    for subject in ["Sorted probe C", "Sorted probe A", "Sorted probe B"] {
        let raw = format!(
            "From: sortable@example.com\r\nTo: test@example.com\r\nSubject: {subject}\r\nMessage-ID: <{subject}@example.com>\r\nDate: 2026-04-14 09:00:00+00:00\r\n\r\norder"
        );
        next::append(&mut client, FOLDER, vec![], raw.as_bytes()).expect("append");
    }
    client.logout();

    let pool = Arc::new(SessionPool::with_credentials(Arc::new(FixedCredentials)));
    let service = ImapSmtpService::new(account, pool);
    let order = SortOrder {
        key: SortKey::Subject,
        descending: false,
    };
    let query = Some("subject Sorted");

    let first = service
        .list_envelopes_sorted(None, FOLDER, 1, 2, query, order)
        .expect("page one");
    assert_eq!(first.len(), 2, "the page holds two of the three probes");
    assert_eq!(first[0].subject, "Sorted probe A");
    assert_eq!(first[1].subject, "Sorted probe B");

    let second = service
        .list_envelopes_sorted(None, FOLDER, 2, 2, query, order)
        .expect("page two");
    assert_eq!(second.len(), 1);
    assert_eq!(
        second[0].subject, "Sorted probe C",
        "the ordering continued into the next page"
    );
}
