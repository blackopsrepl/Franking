/*! Unit tests for the codec-based mapping and search grammar. */

use super::map::{role_from_names, uids_from_search};
use super::search::criteria;
use crate::mail::session::{CommandOutput, ResponseReader};
use crate::mail::types::FolderRole;
use imap_types::search::SearchKey;

/// Read a script of responses, decoding until the input is exhausted.
struct Script<'a> {
    data: &'a [u8],
    position: usize,
}

impl std::io::Read for Script<'_> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let remaining = &self.data[self.position..];
        let take = remaining.len().min(buf.len());
        buf[..take].copy_from_slice(&remaining[..take]);
        self.position += take;
        Ok(take)
    }
}

impl std::io::Write for Script<'_> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn output_from(script: &str) -> CommandOutput {
    let mut reader = ResponseReader::new(Script {
        data: script.as_bytes(),
        position: 0,
    });
    let mut output = CommandOutput::default();
    while let Ok(response) = reader.next_response() {
        output.responses.push(response);
    }
    output
}

#[test]
fn maps_a_list_response_to_folders_with_roles() {
    let output = output_from(
        "* LIST (\\HasNoChildren \\Sent) \"/\" \"Sent\"\r\n\
         * LIST (\\HasNoChildren) \"/\" \"Archive\"\r\n\
         * LIST (\\Noselect \\HasChildren) \"/\" \"[Gmail]\"\r\n\
         a001 OK List completed\r\n",
    );
    let folders = super::map::folders_from_list(&output);
    assert_eq!(folders.len(), 2, "unselectable mailboxes are dropped");
    assert_eq!(folders[0].name, "Sent");
    assert_eq!(folders[0].role, FolderRole::Sent);
    assert_eq!(folders[1].name, "Archive");
    assert_eq!(folders[1].role, FolderRole::Other);
}

#[test]
fn maps_a_fetch_response_to_envelopes() {
    let output = output_from(
        "* 1 FETCH (UID 7 FLAGS (\\Seen) INTERNALDATE \"13-Apr-2026 09:00:00 +0000\" ENVELOPE (\"Mon, 13 Apr 2026 09:00:00 +0000\" \"Subject line\" ((\"Alice\" NIL \"alice\" \"example.com\")) NIL NIL ((NIL NIL \"bob\" \"example.com\")) NIL NIL NIL \"<abc@example.com>\"))\r\n\
         a001 OK Fetch completed\r\n",
    );
    let envelopes = super::map::envelopes_from_fetch(&output);
    assert_eq!(envelopes.len(), 1);
    let envelope = &envelopes[0];
    assert_eq!(envelope.id, "7");
    assert_eq!(envelope.subject, "Subject line");
    assert_eq!(envelope.message_id.as_deref(), Some("abc@example.com"));
    assert!(envelope.flags.iter().any(|flag| flag == "Seen"));
    assert!(envelope.date.contains("2026-04-13"));
    assert_eq!(envelope.sender_display(), "Alice", "the display name wins");
}

#[test]
fn maps_sort_and_thread_responses_to_uids() {
    let output = output_from("* SORT 9 4 1\r\na001 OK Sort completed\r\n");
    assert_eq!(uids_from_search(&output), vec![9, 4, 1]);

    let output = output_from("* THREAD (1)(2 3)\r\na001 OK Thread completed\r\n");
    let groups = super::map::thread_groups(&output);
    assert_eq!(groups, vec![vec![1], vec![2, 3]]);
}

#[test]
fn recognises_every_special_use_role() {
    for (name, role) in [
        ("\\Sent", FolderRole::Sent),
        ("\\Drafts", FolderRole::Drafts),
        ("\\Trash", FolderRole::Trash),
        ("\\Archive", FolderRole::Archive),
        ("\\Junk", FolderRole::Junk),
        ("\\Flagged", FolderRole::Flagged),
        ("\\All", FolderRole::All),
    ] {
        assert_eq!(role_from_names(&[name.to_string()]), role, "{name}");
    }
    assert_eq!(
        role_from_names(&["\\HasNoChildren".to_string()]),
        FolderRole::Other
    );
}

#[test]
fn translates_the_search_grammar_to_typed_keys() {
    assert_eq!(criteria(None), imap_types::core::Vec1::from(SearchKey::All));
    assert_eq!(
        criteria(Some("flag seen")),
        imap_types::core::Vec1::from(SearchKey::Seen)
    );

    let keys = criteria(Some("subject quarterly and not flag flagged"));
    assert_eq!(keys.as_ref().len(), 2);
    assert!(matches!(keys.as_ref()[0], SearchKey::Subject(_)));
    assert!(matches!(keys.as_ref()[1], SearchKey::Unflagged));

    let keys = criteria(Some("body standing order"));
    match &keys.as_ref()[0] {
        SearchKey::Body(text) => {
            assert_eq!(text.as_ref(), b"standing order");
        }
        other => panic!("expected BODY, got {other:?}"),
    }

    let keys = criteria(Some("revenue"));
    assert!(matches!(keys.as_ref()[0], SearchKey::Text(_)));
}
