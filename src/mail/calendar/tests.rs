use super::{parse_invitation, Event};

fn ics(body: &str) -> String {
    format!("BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\n{body}\r\nEND:VEVENT\r\nEND:VCALENDAR")
}

#[test]
fn extracts_event_fields() {
    let event = parse_invitation(&ics(
        "SUMMARY:Team sync\r\nDTSTART;TZID=UTC:20260413T090000\r\nDTEND:20260413T093000\r\nLOCATION:Room 1",
    ))
    .unwrap();

    assert_eq!(event.summary.as_deref(), Some("Team sync"));
    assert_eq!(event.start.as_deref(), Some("20260413T090000"));
    assert_eq!(event.start_tz.as_deref(), Some("UTC"));
    assert_eq!(event.location.as_deref(), Some("Room 1"));
}

#[test]
fn a_utc_start_is_converted_to_the_reader_timezone() {
    let event = parse_invitation(&ics("SUMMARY:Standup\r\nDTSTART:20260413T090000Z")).unwrap();
    let display = event.start_display().expect("a display time");

    // The same instant on the reader's clock, with the timezone named.
    let expected = chrono::DateTime::parse_from_rfc3339("2026-04-13T09:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Local)
        .format("%Y-%m-%d %H:%M")
        .to_string();
    assert_eq!(display, format!("{expected} (UTC)"), "{display}");
}

#[test]
fn a_named_timezone_start_is_converted_and_named() {
    let event = parse_invitation(&ics(
        "SUMMARY:Review\r\nDTSTART;TZID=Europe/Berlin:20260413T090000",
    ))
    .unwrap();
    assert_eq!(event.start_tz.as_deref(), Some("Europe/Berlin"));
    let display = event.start_display().expect("a display time");
    assert!(display.ends_with("(Europe/Berlin)"), "{display}");
    assert!(
        display.contains("2026-04-13"),
        "the instant keeps its date in the reader's zone: {display}"
    );
}

#[test]
fn a_bare_local_time_is_shown_without_inventing_a_zone() {
    let event = parse_invitation(&ics("SUMMARY:Local\r\nDTSTART:20260413T090000")).unwrap();
    assert_eq!(event.start_tz, None);
    assert_eq!(event.start_display().as_deref(), Some("2026-04-13 09:00"));
}

#[test]
fn an_all_day_event_is_labelled() {
    let event = parse_invitation(&ics("SUMMARY:Holiday\r\nDTSTART;VALUE=DATE:20260413")).unwrap();
    assert!(event.all_day);
    assert_eq!(
        event.start_display().as_deref(),
        Some("2026-04-13 (all day)")
    );
}

#[test]
fn an_unknown_timezone_still_reports_the_identifier() {
    let event = parse_invitation(&ics(
        "SUMMARY:Odd\r\nDTSTART;TZID=Mars/Olympus:20260413T090000",
    ))
    .unwrap();
    let display = event.start_display().expect("a display time");
    assert_eq!(display, "2026-04-13 09:00 (Mars/Olympus)");
}

#[test]
fn returns_none_without_event() {
    assert!(parse_invitation("BEGIN:VCALENDAR\r\nEND:VCALENDAR").is_none());
}

#[test]
fn an_unparseable_start_is_shown_verbatim() {
    let event = Event {
        start: Some("whenever".to_string()),
        ..Event::default()
    };
    assert_eq!(event.start_display().as_deref(), Some("whenever"));
}

#[test]
fn a_cancelled_invitation_is_marked_and_not_repliable() {
    let event = parse_invitation(
        "BEGIN:VCALENDAR\r\nMETHOD:CANCEL\r\nBEGIN:VEVENT\r\nUID:1\r\nORGANIZER:mailto:a@example.com\r\nSUMMARY:Team sync\r\nDTSTART:20260413T090000Z\r\nEND:VEVENT\r\nEND:VCALENDAR",
    )
    .unwrap();

    assert!(event.cancelled);
    let summary = event.summary_line().expect("a summary");
    assert!(summary.ends_with("(cancelled)"), "{summary}");
}

#[test]
fn a_cancelled_status_is_treated_as_a_cancellation() {
    let event = parse_invitation(&ics(
        "UID:1\r\nORGANIZER:mailto:a@example.com\r\nSTATUS:CANCELLED\r\nSUMMARY:Gone",
    ))
    .unwrap();
    assert!(event.cancelled);
}
