use super::ResponseReader;
use imap_types::response::{Code, Data, Response, StatusKind};

/// The exact dialogue captured from Dovecot 2.x, including the lines that made
/// the legacy `imap` crate abort and panic.
const TRANSCRIPT: &[&str] = &[
    "* OK [CAPABILITY IMAP4rev1 SASL-IR LOGIN-REFERRALS ID ENABLE IDLE SORT SORT=DISPLAY THREAD=REFERENCES UIDPLUS NAMESPACE CONDSTORE QRESYNC ESEARCH ESORT MOVE COMPRESS=DEFLATE INPROGRESS NOTIFY LITERAL+ UTF8=ACCEPT FILTER=SIEVE] Dovecot ready.\r\n",
    "a001 OK [CAPABILITY IMAP4rev1 SORT THREAD=REFERENCES UIDPLUS CONDSTORE QRESYNC ESEARCH MOVE UTF8=ACCEPT] Logged in\r\n",
    "* ENABLED QRESYNC\r\n",
    "a004 OK [APPENDUID 1790003434 1] Append completed\r\n",
    "* FLAGS (\\Answered \\Flagged \\Deleted \\Seen \\Draft)\r\n",
    "* OK [PERMANENTFLAGS (\\Answered \\Flagged \\Deleted \\Seen \\Draft \\*)] Flags permitted.\r\n",
    "* 2 EXISTS\r\n",
    "* 2 RECENT\r\n",
    "* OK [UNSEEN 1] First unseen.\r\n",
    "* OK [UIDVALIDITY 1790003434] UIDs valid\r\n",
    "* OK [UIDNEXT 3] Predicted next UID\r\n",
    "* OK [HIGHESTMODSEQ 3] Highest\r\n",
    "a006 OK [READ-WRITE] Select completed\r\n",
    "* SORT 1 2\r\n",
    "a007 OK Sort completed\r\n",
    "* THREAD (1)(2)\r\n",
    "a008 OK Thread completed\r\n",
    "* 1 FETCH (UID 1 FLAGS (\\Recent) MODSEQ (2))\r\n",
    "* OK [COPYUID 1790003435 1 1] Moved UIDs.\r\n",
    "* VANISHED 1\r\n",
    "a010 OK [HIGHESTMODSEQ 4] Move completed\r\n",
    "* 1 FETCH (UID 2 MODSEQ (5) FLAGS (\\Seen \\Recent))\r\n",
    "* 1 EXISTS\r\n",
    "* BYE Logging out\r\n",
];

fn reader(input: &[u8]) -> ResponseReader<&[u8]> {
    ResponseReader::new(input)
}

#[test]
fn decodes_every_line_the_legacy_client_choked_on() {
    let transcript = TRANSCRIPT.concat();
    let mut reader = reader(transcript.as_bytes());
    let mut seen_sort = false;
    let mut seen_thread = false;
    let mut seen_vanished = false;
    let mut seen_enabled = false;
    let mut seen_modseq_fetch = false;
    let mut highest_modseq = false;
    let mut append_uid = false;
    let mut copy_uid = false;

    for _ in 0..TRANSCRIPT.len() {
        let response = reader.next_response().expect("every real line decodes");
        match response {
            Response::Data(Data::Sort(ids, _)) => {
                seen_sort = true;
                assert_eq!(
                    ids.iter().map(|id| id.get()).collect::<Vec<_>>(),
                    vec![1, 2]
                );
            }
            Response::Data(Data::Thread(threads)) => {
                seen_thread = true;
                assert_eq!(threads.len(), 2);
            }
            Response::Data(Data::Vanished { earlier, .. }) => {
                seen_vanished = true;
                assert!(!earlier);
            }
            Response::Data(Data::Enabled { .. }) => seen_enabled = true,
            Response::Data(Data::Fetch { items, .. }) => {
                if items
                    .as_ref()
                    .iter()
                    .any(|item| matches!(item, imap_types::fetch::MessageDataItem::ModSeq(_)))
                {
                    seen_modseq_fetch = true;
                }
            }
            Response::Status(status) => {
                let code = match status {
                    imap_types::response::Status::Tagged(tagged) => tagged.body.code,
                    imap_types::response::Status::Untagged(body) => body.code,
                    imap_types::response::Status::Bye(_) => None,
                };
                match code {
                    Some(Code::AppendUid { .. }) => append_uid = true,
                    Some(Code::CopyUid { .. }) => copy_uid = true,
                    Some(Code::HighestModSeq(_)) => highest_modseq = true,
                    _ => {}
                }
            }
            _ => {}
        }
    }

    assert!(seen_sort, "untagged SORT decoded");
    assert!(seen_thread, "untagged THREAD decoded");
    assert!(seen_vanished, "untagged VANISHED decoded");
    assert!(seen_enabled, "untagged ENABLED decoded");
    assert!(seen_modseq_fetch, "FETCH with MODSEQ decoded");
    assert!(highest_modseq, "HIGHESTMODSEQ response code decoded");
    assert!(append_uid, "APPENDUID response code decoded");
    assert!(copy_uid, "COPYUID response code decoded");
    assert!(reader.skipped.is_empty(), "nothing was skipped");
}

#[test]
fn skips_undecodable_lines_without_desynchronizing() {
    let input = "* WHO KNOWS what this is\r\n\
                 * SORT 7 8\r\n\
                 * 1 FETCH (UID 1 BOGUS)\r\n\
                 a001 OK Sort completed\r\n";
    let mut reader = reader(input.as_bytes());

    let first = reader.next_response().expect("decodes after junk");
    match first {
        Response::Data(Data::Sort(ids, _)) => {
            assert_eq!(
                ids.iter().map(|id| id.get()).collect::<Vec<_>>(),
                vec![7, 8]
            );
        }
        other => panic!("expected SORT, got {other:?}"),
    }

    let output = reader.collect("a001").expect("collect");
    assert!(output.completion.as_ref().expect("completion").is_ok());
    assert_eq!(output.skipped.len(), 2, "both junk lines were counted");
}

#[test]
fn parses_when_the_socket_delivers_one_byte_at_a_time() {
    struct Drip<'a> {
        data: &'a [u8],
        position: usize,
    }
    impl std::io::Read for Drip<'_> {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            if self.position >= self.data.len() || buf.is_empty() {
                return Ok(0);
            }
            buf[0] = self.data[self.position];
            self.position += 1;
            Ok(1)
        }
    }

    let transcript = "* SORT 1 2 3\r\na001 OK done\r\n";
    let mut reader = ResponseReader::new(Drip {
        data: transcript.as_bytes(),
        position: 0,
    });
    let response = reader.next_response().expect("decodes across reads");
    assert!(matches!(response, Response::Data(Data::Sort(..))));
    let output = reader.collect("a001").expect("collect");
    assert!(output.completion.expect("completion").is_ok());
}

#[test]
fn decodes_a_response_carrying_a_literal() {
    let input = b"* 1 FETCH (UID 1 BODY[] {5}\r\nhello)\r\na001 OK done\r\n";
    let mut reader = reader(input);
    let response = reader.next_response().expect("literal response decodes");
    match response {
        Response::Data(Data::Fetch { items, .. }) => {
            assert!(!items.as_ref().is_empty());
        }
        other => panic!("expected FETCH, got {other:?}"),
    }
    let output = reader.collect("a001").expect("collect");
    assert!(output.completion.expect("completion").is_ok());
}

#[test]
fn reports_errors_from_untagged_status_and_completion() {
    let input = "* 1 EXPUNGE\r\na001 NO [TRYCREATE] try again\r\n";
    let mut reader = reader(input.as_bytes());
    let output = reader.collect("a001").expect("collect");
    {
        let completion = output.completion.as_ref().expect("completion");
        assert!(!completion.is_ok());
        assert!(completion.code_debug().contains("TryCreate"));
    }

    let error = output.require_ok("SELECT").expect_err("must fail");
    assert!(error.to_string().contains("try again"));
}

#[test]
fn reports_a_dropped_connection_mid_response() {
    let mut reader = reader(b"* SORT 1");
    let error = reader.next_response().expect_err("truncated input fails");
    assert!(error.to_string().contains("mid-response"));
}

#[test]
fn collects_untagged_status_lines() {
    let input = "* OK [UIDNEXT 5] predicted\r\na001 OK done\r\n";
    let mut reader = reader(input.as_bytes());
    let output = reader.collect("a001").expect("collect");
    assert_eq!(output.untagged_status, vec!["predicted".to_string()]);
    assert!(matches!(
        output.completion.map(|c| c.status),
        Some(StatusKind::Ok)
    ));
}

#[test]
fn rejects_a_response_tagged_for_another_command() {
    let mut reader = reader(b"a999 OK wrong command\r\n");
    let error = reader
        .collect("a001")
        .expect_err("mismatched tag is a protocol error");
    assert!(error.to_string().contains("does not match command tag"));
}
