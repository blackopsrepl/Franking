//! Report and calendar-list parsing.

use super::{parse_default_calendar, parse_import};

#[test]
fn reads_an_import_report() {
    let report = parse_import(
        r#"{"data":{"calendar_name":"Personal","imported":1,"skipped":0,
                "warnings":["ignored unsupported attendee data"]},"status":"ok"}"#,
    )
    .expect("report");

    assert_eq!(report.calendar_name, "Personal");
    assert_eq!(report.imported, 1);
    assert_eq!(report.skipped, 0);
    assert!(report.summary().contains("Added 1 event(s)"));
    assert!(report
        .summary()
        .contains("ignored unsupported attendee data"));
}

#[test]
fn counts_skipped_events_in_the_summary() {
    let report = parse_import(
        r#"{"data":{"calendar_name":"Work","imported":2,"skipped":1,"warnings":[]},"status":"ok"}"#,
    )
    .expect("report");
    assert!(
        report.summary().contains("1 skipped"),
        "{}",
        report.summary()
    );
}

#[test]
fn a_failed_import_reports_the_error() {
    let error =
        parse_import(r#"{"status":"error","error":"calendar not found"}"#).expect_err("refused");
    assert!(error.to_string().contains("calendar not found"), "{error}");
}

#[test]
fn output_that_is_not_json_is_reported_as_such() {
    let error = parse_import("not json").expect_err("refused");
    assert!(error.to_string().contains("no JSON report"), "{error}");
}

#[test]
fn the_default_calendar_is_preferred() {
    let id = parse_default_calendar(
        r#"{"data":[{"id":"a","is_default":false},{"id":"b","is_default":true}]}"#,
    )
    .expect("a calendar");
    assert_eq!(id, "b");
}

#[test]
fn the_first_calendar_is_used_when_none_is_marked() {
    let id = parse_default_calendar(r#"{"data":[{"id":"a"},{"id":"b"}]}"#).expect("a calendar");
    assert_eq!(id, "a");
}

#[test]
fn no_calendars_is_an_error() {
    let error = parse_default_calendar(r#"{"data":[]}"#).expect_err("refused");
    assert!(error.to_string().contains("no calendar"), "{error}");
}
