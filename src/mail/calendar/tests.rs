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
fn a_utc_start_is_shown_with_its_zone_and_the_local_equivalent() {
    let event = parse_invitation(&ics("SUMMARY:Standup\r\nDTSTART:20260413T090000Z")).unwrap();
    let display = event.start_display_in("Europe/Berlin").expect("a time");

    // The organizer's time keeps its own label, and the reader's equivalent is
    // named with the reader's zone, so neither number can be misread.
    assert_eq!(
        display,
        "2026-04-13 09:00 UTC (2026-04-13 11:00 Europe/Berlin)"
    );
}

#[test]
fn a_named_timezone_start_is_shown_as_written_and_converted() {
    let event = parse_invitation(&ics(
        "SUMMARY:Review\r\nDTSTART;TZID=Europe/Berlin:20260416T100000",
    ))
    .unwrap();
    assert_eq!(event.start_tz.as_deref(), Some("Europe/Berlin"));
    let display = event.start_display_in("Europe/Rome").expect("a time");
    assert_eq!(
        display,
        "2026-04-16 10:00 Europe/Berlin (2026-04-16 10:00 Europe/Rome)"
    );
}

#[test]
fn a_start_in_the_reader_zone_needs_no_conversion() {
    let event = parse_invitation(&ics(
        "SUMMARY:Local\r\nDTSTART;TZID=Europe/Berlin:20260416T100000",
    ))
    .unwrap();
    let display = event.start_display_in("Europe/Berlin").expect("a time");
    assert_eq!(display, "2026-04-16 10:00 Europe/Berlin");
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
    assert_eq!(
        display, "2026-04-13 09:00 Mars/Olympus",
        "an unknown zone is named, not assumed"
    );
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
