/*! App unit tests: calendar invitation replies. */

#[test]
fn replying_to_an_invitation_drafts_a_calendar_reply() {
    use crate::keys::View;
    use crate::mail::calendar_reply::PartStat;

    use super::super::App;

    let raw = b"From: alice@example.com\r\nSubject: Invitation\r\nContent-Type: text/calendar; method=REQUEST\r\n\r\nBEGIN:VCALENDAR\r\nVERSION:2.0\r\nMETHOD:REQUEST\r\nBEGIN:VEVENT\r\nUID:invite-1@example.com\r\nORGANIZER:mailto:alice@example.com\r\nSUMMARY:Standup\r\nDTSTART:20260921T090000Z\r\nDTEND:20260921T093000Z\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
    let mut document = crate::mail::mime::parse_message(raw).unwrap();
    document.raw = Some(raw.to_vec());

    let mut app = App::new(None);
    app.view = View::MessageView;
    app.message_content = Some(document);

    app.open_invite_reply();
    assert_eq!(app.view, View::InviteReply);
    assert!(app.invite_pending.is_some());

    app.respond_invitation(PartStat::Accepted);
    assert_eq!(app.view, View::Compose);
    assert!(app.status_message.contains("Review the calendar reply"));

    let cs = app.compose_state.as_ref().expect("compose");
    assert_eq!(cs.to, "alice@example.com");
    assert!(cs.subject.contains("Accepted"));
    assert_eq!(cs.attachments.len(), 1);
    assert!(cs.attachments[0].ends_with(".reply.ics"));

    let reply = std::fs::read_to_string(&cs.attachments[0]).expect("reply file");
    assert!(reply.contains("METHOD:REPLY"));
    assert!(reply.contains("PARTSTAT=ACCEPTED"));
    let _ = std::fs::remove_file(&cs.attachments[0]);
}

#[test]
fn invitations_without_a_uid_are_not_repliable() {
    use crate::keys::View;

    use super::super::App;

    let raw = b"From: alice@example.com\r\nSubject: Note\r\nContent-Type: text/calendar; method=REQUEST\r\n\r\nBEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nSUMMARY:Undated\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
    let mut document = crate::mail::mime::parse_message(raw).unwrap();
    document.raw = Some(raw.to_vec());

    let mut app = App::new(None);
    app.view = View::MessageView;
    app.message_content = Some(document);
    app.open_invite_reply();
    assert_eq!(app.view, View::MessageView);
    assert!(app.status_message.contains("no replyable invitation"));
}

#[test]
fn cancelling_the_invite_prompt_returns_to_the_message() {
    use crate::keys::View;

    use super::super::App;

    let raw = b"From: alice@example.com\r\nSubject: Invitation\r\nContent-Type: text/calendar; method=REQUEST\r\n\r\nBEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:invite-2@example.com\r\nORGANIZER:mailto:alice@example.com\r\nSUMMARY:Standup\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
    let mut document = crate::mail::mime::parse_message(raw).unwrap();
    document.raw = Some(raw.to_vec());

    let mut app = App::new(None);
    app.view = View::MessageView;
    app.message_content = Some(document);
    app.open_invite_reply();
    assert_eq!(app.view, View::InviteReply);
    app.cancel_invite_reply();
    assert_eq!(app.view, View::MessageView);
    assert!(app.invite_pending.is_none());
}
