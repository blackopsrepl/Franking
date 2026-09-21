//! Command-layer behavior and the legacy equivalence gate.

use std::sync::Arc;

use imap_types::command::{CommandBody, FetchModifier, SelectParameter};
use imap_types::core::{Atom, Charset, Vec1};
use imap_types::extensions::sort::{SortCriterion, SortKey};
use imap_types::extensions::thread::ThreadingAlgorithm;
use imap_types::fetch::{MacroOrMessageDataItemNames, MessageDataItemName};
use imap_types::response::{Code, Data, Response};
use imap_types::search::SearchKey;
use imap_types::sequence::SequenceSet;
use solverforge_mail::mail::remote::next;
use solverforge_mail::mail::session::{open_imap_client, SessionPool};

use super::support::{account, mailbox_lock, seed, test_address, FixedCredentials};

#[test]
fn sorts_threads_and_tracks_modseq_over_the_codec_layer() {
    let Some((host, port)) = test_address() else {
        return;
    };
    let _guard = mailbox_lock();
    let account = account(&host, port);
    seed(&account);

    let credentials = FixedCredentials;
    let mut client =
        open_imap_client(&account, &credentials).expect("connect with the codec client");

    let capabilities = client.capability().expect("capability");
    let names: Vec<String> = capabilities.iter().map(|cap| format!("{cap:?}")).collect();
    assert!(
        names.iter().any(|name| name.contains("Sort")),
        "server advertises SORT: {names:?}"
    );
    assert!(
        names.iter().any(|name| name.contains("QResync")),
        "server advertises QRESYNC: {names:?}"
    );
    assert!(names.iter().any(|name| name.contains("CondStore")));
    assert!(names.iter().any(|name| name.contains("UidPlus")));
    assert!(names.iter().any(|name| name.contains("Move")));
    assert!(names.iter().any(|name| name.contains("Idle")));

    let select = client
        .run(CommandBody::Select {
            mailbox: imap_types::mailbox::Mailbox::Inbox,
            parameters: vec![SelectParameter::CondStore],
        })
        .expect("select")
        .require_ok("SELECT")
        .expect("select ok");
    let highest_modseq = select.untagged_code(|code| match code {
        Code::HighestModSeq(value) => Some(*value),
        _ => None,
    });
    assert!(highest_modseq.is_some(), "SELECT reported HIGHESTMODSEQ");

    let sort = client
        .run(CommandBody::Sort {
            sort_criteria: Vec1::from(SortCriterion {
                key: SortKey::Date,
                reverse: false,
            }),
            charset: Charset::Atom(Atom::try_from("UTF-8").unwrap()),
            search_criteria: Vec1::from(SearchKey::All),
            uid: true,
        })
        .expect("sort")
        .require_ok("UID SORT")
        .expect("sort ok");
    let sorted = sort.responses.iter().find_map(|response| match response {
        Response::Data(Data::Sort(ids, _)) => {
            Some(ids.iter().map(|id| id.get()).collect::<Vec<u32>>())
        }
        _ => None,
    });
    let sorted = sorted.expect("untagged SORT response");
    assert!(sorted.len() >= 2, "sorted every seeded message: {sorted:?}");

    let thread = client
        .run(CommandBody::Thread {
            algorithm: ThreadingAlgorithm::References,
            charset: Charset::Atom(Atom::try_from("UTF-8").unwrap()),
            search_criteria: Vec1::from(SearchKey::All),
            uid: true,
        })
        .expect("thread")
        .require_ok("UID THREAD")
        .expect("thread ok");
    let threads = thread.responses.iter().find_map(|response| match response {
        Response::Data(Data::Thread(threads)) => Some(threads.len()),
        _ => None,
    });
    assert!(threads.is_some(), "untagged THREAD response decoded");

    let changed = client
        .run(CommandBody::Fetch {
            sequence_set: SequenceSet::try_from("1:*").unwrap(),
            macro_or_item_names: MacroOrMessageDataItemNames::MessageDataItemNames(vec![
                MessageDataItemName::Flags,
            ]),
            uid: true,
            modifiers: vec![FetchModifier::ChangedSince(
                std::num::NonZeroU64::new(1).unwrap(),
            )],
        })
        .expect("fetch with CHANGEDSINCE")
        .require_ok("UID FETCH CHANGEDSINCE")
        .expect("fetch ok");
    let has_modseq = changed.responses.iter().any(|response| match response {
        Response::Data(Data::Fetch { items, .. }) => items
            .as_ref()
            .iter()
            .any(|item| matches!(item, imap_types::fetch::MessageDataItem::ModSeq(_))),
        _ => false,
    });
    assert!(has_modseq, "FETCH returned MODSEQ data");

    assert!(
        client.skipped_lines().is_empty(),
        "no response line was skipped: {:?}",
        client.skipped_lines()
    );
    client.logout();
}

/// The migration gate: the codec-based layer must return exactly what the
/// legacy client returns for the same folders and messages.
#[test]
fn codec_layer_matches_the_legacy_client() {
    let Some((host, port)) = test_address() else {
        return;
    };
    let account = account(&host, port);
    seed(&account);

    let credentials = FixedCredentials;
    let legacy = solverforge_mail::mail::remote::ImapSmtpService::new(
        account.clone(),
        Arc::new(SessionPool::with_credentials(Arc::new(FixedCredentials))),
    );
    let mut client = open_imap_client(&account, &credentials).expect("codec client");

    // Folders, including special-use roles.
    let new_folders = next::list_folders(&mut client).expect("new LIST");
    let old_folders = legacy.list_folders(None).expect("legacy LIST");
    let new_names: Vec<(String, String)> = new_folders
        .iter()
        .map(|folder| (folder.name.clone(), format!("{:?}", folder.role)))
        .collect();
    let old_names: Vec<(String, String)> = old_folders
        .iter()
        .map(|folder| (folder.name.clone(), format!("{:?}", folder.role)))
        .collect();
    assert_eq!(new_names, old_names, "folder names and roles agree");

    // Envelopes for every message in INBOX.
    next::select(&mut client, "INBOX").expect("select");
    let uids = next::search_uids(&mut client, next::search::criteria(None)).expect("search");
    let new_envelopes = next::fetch_envelopes(&mut client, &uids).expect("new FETCH");
    let old_envelopes = legacy
        .list_envelopes(None, "INBOX", 1, 1000, None)
        .expect("legacy listing");

    let summarize = |envelopes: &[solverforge_mail::mail::types::Envelope]| {
        let mut rows: Vec<(String, String, String, bool, bool)> = envelopes
            .iter()
            .map(|envelope| {
                (
                    envelope.subject.clone(),
                    envelope.sender_display(),
                    envelope.date.clone(),
                    envelope.is_seen(),
                    envelope.is_flagged(),
                )
            })
            .collect();
        rows.sort();
        rows
    };
    assert_eq!(
        summarize(&new_envelopes),
        summarize(&old_envelopes),
        "envelope metadata agrees"
    );
    assert!(
        !new_envelopes.is_empty(),
        "the inbox has messages to compare"
    );

    // Server-side ordering: newest first, which the legacy client cannot ask for.
    let sorted = next::sort_uids(
        &mut client,
        Vec1::from(SortCriterion {
            key: SortKey::Date,
            reverse: true,
        }),
        Vec1::from(SearchKey::All),
    )
    .expect("SORT");
    assert!(!sorted.is_empty());

    // Raw body read, byte-identical to the legacy path.
    let uid = uids.iter().copied().max().expect("a uid");
    let new_raw = next::read_message_raw(&mut client, uid).expect("new raw read");
    let old_raw = legacy
        .read_message_raw(None, "INBOX", &uid.to_string())
        .expect("legacy raw read");
    assert_eq!(new_raw, old_raw, "raw message bytes agree");

    client.logout();
}
